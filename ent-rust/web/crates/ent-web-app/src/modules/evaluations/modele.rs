//! Logique d'écran pure des évaluations (S2-7) : lecture d'une cellule de la grille,
//! calcul des seules modifications à envoyer, libellés. **Aucun calcul de moyenne** :
//! le serveur les fournit, arrondies selon la règle de l'établissement.

use chrono::NaiveDate;
use ent_types::evaluations::{PeriodeNotesDto, SaisieResultatBody};
use uuid::Uuid;

/// Contenu d'une cellule de la grille.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Saisie {
    Vide,
    Note(f64),
    Absent,
    Dispense,
    NonNote,
}

impl Saisie {
    pub fn statut(self) -> &'static str {
        match self {
            Saisie::Vide => "vide",
            Saisie::Note(_) => "note",
            Saisie::Absent => "absent",
            Saisie::Dispense => "dispense",
            Saisie::NonNote => "non_note",
        }
    }

    pub fn valeur(self) -> Option<f64> {
        match self {
            Saisie::Note(v) => Some(v),
            _ => None,
        }
    }
}

/// Nombre écrit à la française ou à l'anglaise (« 12,5 », « 12.5 »), deux décimales au plus.
pub fn lire_nombre(texte: &str) -> Option<f64> {
    let t = texte.trim().replace(',', ".");
    if t.is_empty() || t.starts_with('+') || t.starts_with('-') {
        return None;
    }
    if let Some((_, dec)) = t.split_once('.') {
        if dec.len() > 2 {
            return None;
        }
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

/// Lit une cellule : nombre (0 ≤ note ≤ barème), « abs », « disp », « nn », ou vide.
/// Le serveur revalide tout ; ce contrôle n'évite qu'un aller-retour inutile.
pub fn lire_saisie(texte: &str, sur: f64) -> Result<Saisie, String> {
    let t = texte.trim().to_lowercase();
    match t.as_str() {
        "" => Ok(Saisie::Vide),
        "a" | "abs" | "absent" => Ok(Saisie::Absent),
        "d" | "disp" | "dispense" | "dispensé" => Ok(Saisie::Dispense),
        "nn" | "n" | "non noté" | "non note" => Ok(Saisie::NonNote),
        _ => match lire_nombre(&t) {
            Some(v) if v <= sur => Ok(Saisie::Note(v)),
            Some(_) => Err(format!("note supérieure au barème ({})", nombre_fr(sur))),
            None => Err("note attendue (ex. 12,5), ou abs, disp, nn".into()),
        },
    }
}

/// Nombre affiché à la française : « 12,5 », « 14 », « 9,25 ».
pub fn nombre_fr(x: f64) -> String {
    let c = (x * 100.0).round() as i64;
    let (ent, dec) = (c / 100, (c % 100).abs());
    if dec == 0 {
        format!("{ent}")
    } else if dec % 10 == 0 {
        format!("{ent},{}", dec / 10)
    } else {
        format!("{ent},{dec:02}")
    }
}

/// Moyenne affichée (« — » si aucune).
pub fn moyenne_fr(x: Option<f64>) -> String {
    x.map(nombre_fr).unwrap_or_else(|| "—".into())
}

/// Texte initial d'une cellule à partir du résultat du serveur.
pub fn texte_resultat(statut: &str, valeur: Option<f64>) -> String {
    match statut {
        "note" => valeur.map(nombre_fr).unwrap_or_default(),
        "absent" => "abs".into(),
        "dispense" => "disp".into(),
        "non_note" => "nn".into(),
        _ => String::new(),
    }
}

/// Une cellule de la grille : (élève, texte initial, texte courant).
pub type Cellule = (Uuid, String, String);

/// Seules les cellules modifiées sont envoyées. En cas d'erreur, rien n'est envoyé et les
/// cellules fautives sont listées (élève, message).
pub fn changements(
    cellules: &[Cellule],
    sur: f64,
) -> Result<Vec<SaisieResultatBody>, Vec<(Uuid, String)>> {
    let mut ok = Vec::new();
    let mut erreurs = Vec::new();
    for (eleve, initial, courant) in cellules {
        if initial.trim().to_lowercase() == courant.trim().to_lowercase() {
            continue;
        }
        // Une saisie équivalente (« 12.5 » au lieu de « 12,5 ») n'est pas un changement.
        if let (Ok(a), Ok(b)) = (lire_saisie(initial, sur), lire_saisie(courant, sur)) {
            if a == b {
                continue;
            }
        }
        match lire_saisie(courant, sur) {
            Ok(s) => ok.push(SaisieResultatBody {
                eleve_id: *eleve,
                statut: s.statut().to_string(),
                valeur: s.valeur(),
            }),
            Err(m) => erreurs.push((*eleve, m)),
        }
    }
    if erreurs.is_empty() {
        Ok(ok)
    } else {
        Err(erreurs)
    }
}

/// Contrôle du formulaire « nouvelle évaluation » : (titre, coefficient, barème).
pub fn lire_devoir(titre: &str, coef: &str, sur: &str) -> Result<(String, f64, f64), String> {
    let titre = titre.trim();
    if titre.is_empty() {
        return Err("Le titre est obligatoire.".into());
    }
    if titre.chars().count() > 200 {
        return Err("Titre trop long (200 caractères au plus).".into());
    }
    let coef = lire_nombre(coef)
        .filter(|c| *c > 0.0 && *c <= 100.0)
        .ok_or("Coefficient attendu entre 0 et 100 (ex. 1 ou 0,5).")?;
    let sur = lire_nombre(sur)
        .filter(|s| *s > 0.0 && *s <= 1000.0)
        .ok_or("Barème attendu (ex. 20).")?;
    Ok((titre.to_string(), coef, sur))
}

/// Période proposée par défaut : celle qui contient le jour, sinon la plus récente.
pub fn periode_par_defaut(periodes: &[PeriodeNotesDto], jour: NaiveDate) -> Option<Uuid> {
    periodes
        .iter()
        .find(|p| p.date_debut <= jour && jour <= p.date_fin)
        .or_else(|| periodes.iter().max_by_key(|p| p.date_debut))
        .map(|p| p.id)
}

pub fn libelle_statut_periode(s: &str) -> &'static str {
    match s {
        "ouverte" => "Ouverte",
        "partielle_appreciations" => "Notes verrouillées, appréciations ouvertes",
        "fermee" => "Fermée",
        _ => "Inconnu",
    }
}

pub fn libelle_statut_bulletin(s: &str) -> &'static str {
    match s {
        "genere" => "Généré (non publié)",
        "publie" => "Publié",
        "remplace" => "Remplacé",
        _ => "Inconnu",
    }
}

/// Mentions du conseil [H : à vérifier] : (code, libellé).
pub const MENTIONS: [(&str, &str); 6] = [
    ("", "Aucune"),
    ("felicitations", "Félicitations"),
    ("complement", "Compliments"),
    ("encouragements", "Encouragements"),
    ("avertissement_travail", "Avertissement (travail)"),
    ("avertissement_conduite", "Avertissement (conduite)"),
];

/// Décisions du conseil [H : à vérifier] : (code, libellé).
pub const DECISIONS: [(&str, &str); 5] = [
    ("", "Aucune"),
    ("passage", "Passage"),
    ("redoublement", "Redoublement"),
    ("orientation", "Orientation"),
    ("reserve", "Décision réservée"),
];

pub const AVIS: [(&str, &str); 4] = [
    ("", "Pas d'avis"),
    ("favorable", "Favorable"),
    ("reserve", "Réservé"),
    ("defavorable", "Défavorable"),
];

pub fn libelle_code(liste: &[(&str, &'static str)], code: Option<&str>) -> &'static str {
    let c = code.unwrap_or("");
    liste
        .iter()
        .find(|(k, _)| *k == c)
        .map(|(_, l)| *l)
        .unwrap_or("—")
}

/// Option vide → `None` (pour l'API).
pub fn option_code(v: &str) -> Option<String> {
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// Nom de fichier proposé pour un export ou un bulletin (lettres, chiffres, tirets).
pub fn nom_fichier(base: &str) -> String {
    let n: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let n = n.trim_matches(|c| c == '-' || c == '.').to_string();
    if n.is_empty() {
        "fichier".into()
    } else {
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_des_cellules() {
        assert_eq!(lire_saisie("12,5", 20.0), Ok(Saisie::Note(12.5)));
        assert_eq!(lire_saisie(" 12.25 ", 20.0), Ok(Saisie::Note(12.25)));
        assert_eq!(lire_saisie("0", 20.0), Ok(Saisie::Note(0.0)));
        assert_eq!(lire_saisie("ABS", 20.0), Ok(Saisie::Absent));
        assert_eq!(lire_saisie("disp", 20.0), Ok(Saisie::Dispense));
        assert_eq!(lire_saisie("NN", 20.0), Ok(Saisie::NonNote));
        assert_eq!(lire_saisie("", 20.0), Ok(Saisie::Vide));
        assert!(lire_saisie("21", 20.0).is_err());
        assert!(lire_saisie("12,345", 20.0).is_err());
        assert!(lire_saisie("-3", 20.0).is_err());
        assert!(lire_saisie("douze", 20.0).is_err());
        assert!(lire_saisie("8", 10.0).is_ok());
    }

    #[test]
    fn seules_les_modifications_partent() {
        let (a, b, c, d) = (
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(3),
            Uuid::from_u128(4),
        );
        let cellules = vec![
            (a, "12,5".to_string(), "12.5".to_string()), // équivalent
            (b, "".to_string(), "abs".to_string()),
            (c, "10".to_string(), "".to_string()), // effacement
            (d, "nn".to_string(), "NN".to_string()),
        ];
        let ch = changements(&cellules, 20.0).unwrap();
        assert_eq!(ch.len(), 2);
        assert_eq!(ch[0].statut, "absent");
        assert_eq!((ch[1].statut.as_str(), ch[1].valeur), ("vide", None));
        let err = changements(&[(a, "".into(), "25".into())], 20.0).unwrap_err();
        assert_eq!(err.len(), 1);
    }

    #[test]
    fn affichage() {
        assert_eq!(texte_resultat("note", Some(12.5)), "12,5");
        assert_eq!(texte_resultat("absent", None), "abs");
        assert_eq!(texte_resultat("inconnu", None), "");
        assert_eq!(moyenne_fr(None), "—");
        assert_eq!(moyenne_fr(Some(15.67)), "15,67");
        assert_eq!(nombre_fr(14.0), "14");
        assert_eq!(
            libelle_code(&MENTIONS, Some("felicitations")),
            "Félicitations"
        );
        assert_eq!(libelle_code(&MENTIONS, None), "Aucune");
        assert_eq!(option_code(" "), None);
        assert_eq!(nom_fichier("lsu 4B/T1.xml"), "lsu-4B-T1.xml");
    }

    #[test]
    fn formulaire_devoir() {
        assert_eq!(
            lire_devoir(" Contrôle ", "0,5", "20"),
            Ok(("Contrôle".into(), 0.5, 20.0))
        );
        assert!(lire_devoir("", "1", "20").is_err());
        assert!(lire_devoir("X", "0", "20").is_err());
        assert!(lire_devoir("X", "1", "0").is_err());
    }

    #[test]
    fn periode_proposee() {
        let p = |id: u128, d: (u32, u32), f: (u32, u32)| PeriodeNotesDto {
            id: Uuid::from_u128(id),
            code: format!("T{id}"),
            libelle: String::new(),
            date_debut: NaiveDate::from_ymd_opt(2026, d.0, d.1).unwrap(),
            date_fin: NaiveDate::from_ymd_opt(2026, f.0, f.1).unwrap(),
            statut: "ouverte".into(),
        };
        let l = vec![p(1, (1, 1), (3, 31)), p(2, (4, 1), (6, 30))];
        let j = NaiveDate::from_ymd_opt(2026, 2, 10).unwrap();
        assert_eq!(periode_par_defaut(&l, j), Some(Uuid::from_u128(1)));
        let j = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        assert_eq!(periode_par_defaut(&l, j), Some(Uuid::from_u128(2)));
        assert_eq!(periode_par_defaut(&[], j), None);
    }
}
