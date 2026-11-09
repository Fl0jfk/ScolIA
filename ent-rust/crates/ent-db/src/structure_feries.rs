//! Socle 5, lot S3 : jours fériés **calculés** par le code (aucun jeu de données externe).
//!
//! Fêtes mobiles dérivées de Pâques (algorithme de Meeus/Jones/Butcher, calendrier grégorien),
//! fêtes fixes nationales, régime local d'Alsace-Moselle et jours fériés propres à certains
//! territoires d'outre-mer. Fonctions pures, sans base ni réseau : testées sans PostgreSQL.
//!
//! **À faire valider (`[NV]`)** : la liste des jours fériés locaux d'outre-mer (dates de
//! l'abolition de l'esclavage) est reprise de la connaissance générale du droit du travail et
//! n'a pas été vérifiée sur une source officielle dans ce lot ; elle est isolée dans
//! [`feries_locaux_outre_mer`] pour être corrigée sans toucher au reste.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Un jour férié calculé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ferie {
    pub date: NaiveDate,
    /// Code stable (ne change jamais : sert à l'unicité et aux tests).
    pub code: &'static str,
    pub libelle: &'static str,
    /// `true` pour un férié de régime local ou de territoire (pas national).
    pub local: bool,
}

/// Dimanche de Pâques (grégorien), `None` hors de 1583..=9999.
pub fn paques(annee: i32) -> Option<NaiveDate> {
    if !(1583..=9999).contains(&annee) {
        return None;
    }
    let a = annee % 19;
    let b = annee / 100;
    let c = annee % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let mois = (h + l - 7 * m + 114) / 31;
    let jour = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(annee, mois as u32, jour as u32)
}

/// Territoire d'application, déduit du code du département du profil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Territoire {
    Metropole,
    Guadeloupe,
    Martinique,
    Guyane,
    Reunion,
    Mayotte,
    SaintBarthelemy,
    SaintMartin,
}

impl Territoire {
    /// Code de département (`971`…`978`) ; tout autre code = métropole.
    pub fn depuis_departement(code: Option<&str>) -> Self {
        match code.map(str::trim) {
            Some("971") => Self::Guadeloupe,
            Some("972") => Self::Martinique,
            Some("973") => Self::Guyane,
            Some("974") => Self::Reunion,
            Some("976") => Self::Mayotte,
            Some("977") => Self::SaintBarthelemy,
            Some("978") => Self::SaintMartin,
            _ => Self::Metropole,
        }
    }
}

/// Alsace-Moselle : régime local explicite du profil, ou départements 57, 67, 68.
pub fn est_alsace_moselle(regime_local: Option<&str>, departement: Option<&str>) -> bool {
    regime_local == Some("alsace_moselle")
        || matches!(departement.map(str::trim), Some("57" | "67" | "68"))
}

/// Jours fériés locaux d'outre-mer (abolition de l'esclavage). **[NV]** voir l'en-tête du module.
pub fn feries_locaux_outre_mer(t: Territoire, annee: i32) -> Option<Ferie> {
    let (mois, jour, code, libelle) = match t {
        Territoire::Metropole => return None,
        Territoire::Mayotte => (4, 27, "abolition_mayotte", "Abolition de l'esclavage (Mayotte)"),
        Territoire::Martinique => (5, 22, "abolition_martinique", "Abolition de l'esclavage (Martinique)"),
        Territoire::Guadeloupe => (5, 27, "abolition_guadeloupe", "Abolition de l'esclavage (Guadeloupe)"),
        Territoire::SaintMartin => (5, 28, "abolition_saint_martin", "Abolition de l'esclavage (Saint-Martin)"),
        Territoire::Guyane => (6, 10, "abolition_guyane", "Abolition de l'esclavage (Guyane)"),
        Territoire::SaintBarthelemy => (10, 9, "abolition_saint_barthelemy", "Abolition de l'esclavage (Saint-Barthélemy)"),
        Territoire::Reunion => (12, 20, "abolition_reunion", "Abolition de l'esclavage (La Réunion)"),
    };
    NaiveDate::from_ymd_opt(annee, mois, jour).map(|date| Ferie {
        date,
        code,
        libelle,
        local: true,
    })
}

/// Jours fériés d'une année civile, triés par date.
pub fn feries(annee: i32, territoire: Territoire, alsace_moselle: bool) -> Vec<Ferie> {
    let mut v: Vec<Ferie> = Vec::new();
    let mut fixe = |mois: u32, jour: u32, code: &'static str, libelle: &'static str| {
        if let Some(date) = NaiveDate::from_ymd_opt(annee, mois, jour) {
            v.push(Ferie {
                date,
                code,
                libelle,
                local: false,
            });
        }
    };
    fixe(1, 1, "jour_an", "Jour de l'an");
    fixe(5, 1, "fete_travail", "Fête du Travail");
    fixe(5, 8, "victoire_1945", "Victoire 1945");
    fixe(7, 14, "fete_nationale", "Fête nationale");
    fixe(8, 15, "assomption", "Assomption");
    fixe(11, 1, "toussaint", "Toussaint");
    fixe(11, 11, "armistice_1918", "Armistice 1918");
    fixe(12, 25, "noel", "Noël");
    if let Some(p) = paques(annee) {
        let mut mobile = |decalage: i64, code: &'static str, libelle: &'static str, local: bool| {
            v.push(Ferie {
                date: p + Duration::days(decalage),
                code,
                libelle,
                local,
            });
        };
        mobile(1, "lundi_paques", "Lundi de Pâques", false);
        mobile(39, "ascension", "Ascension", false);
        // Lundi de Pentecôte : férié, mais « journée de solidarité » travaillée dans certains
        // établissements : la direction peut l'annuler (événement `calcule` annulé, motif).
        mobile(50, "lundi_pentecote", "Lundi de Pentecôte", false);
        if alsace_moselle {
            mobile(-2, "vendredi_saint", "Vendredi saint (Alsace-Moselle)", true);
        }
    }
    if alsace_moselle {
        if let Some(date) = NaiveDate::from_ymd_opt(annee, 12, 26) {
            v.push(Ferie {
                date,
                code: "saint_etienne",
                libelle: "Saint-Étienne (Alsace-Moselle)",
                local: true,
            });
        }
    }
    if let Some(f) = feries_locaux_outre_mer(territoire, annee) {
        v.push(f);
    }
    v.sort_by_key(|f| f.date);
    v
}

/// Fériés dont la date tombe dans `[du, au]` (couvre les deux années civiles d'une année scolaire).
pub fn feries_entre(
    du: NaiveDate,
    au: NaiveDate,
    territoire: Territoire,
    alsace_moselle: bool,
) -> Vec<Ferie> {
    (du.year()..=au.year())
        .flat_map(|a| feries(a, territoire, alsace_moselle))
        .filter(|f| f.date >= du && f.date <= au)
        .collect()
}

/// Ponts possibles : le lundi avant un mardi férié, le vendredi après un jeudi férié.
/// Suggestion seulement (la direction décide) ; la date proposée n'est jamais un week-end.
pub fn ponts_possibles(feries: &[Ferie]) -> Vec<(NaiveDate, NaiveDate)> {
    feries
        .iter()
        .filter_map(|f| match f.date.weekday() {
            Weekday::Tue => Some((f.date, f.date - Duration::days(1))),
            Weekday::Thu => Some((f.date, f.date + Duration::days(1))),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn paques_connues() {
        assert_eq!(paques(2024), Some(d(2024, 3, 31)));
        assert_eq!(paques(2025), Some(d(2025, 4, 20)));
        assert_eq!(paques(2026), Some(d(2026, 4, 5)));
        assert_eq!(paques(2027), Some(d(2027, 3, 28)));
        assert_eq!(paques(2038), Some(d(2038, 4, 25)));
        assert_eq!(paques(1583 - 1), None);
    }

    #[test]
    fn feries_metropole_2027_avec_mobiles() {
        let f = feries(2027, Territoire::Metropole, false);
        let get = |code: &str| f.iter().find(|x| x.code == code).map(|x| x.date);
        assert_eq!(f.len(), 11);
        assert_eq!(get("lundi_paques"), Some(d(2027, 3, 29)));
        assert_eq!(get("ascension"), Some(d(2027, 5, 6)));
        assert_eq!(get("lundi_pentecote"), Some(d(2027, 5, 17)));
        assert!(f.windows(2).all(|w| w[0].date <= w[1].date));
        assert!(f.iter().all(|x| !x.local));
    }

    #[test]
    fn alsace_moselle_et_outre_mer() {
        let am = feries(2027, Territoire::Metropole, true);
        assert_eq!(am.len(), 13);
        assert!(am.iter().any(|x| x.code == "vendredi_saint" && x.date == d(2027, 3, 26)));
        assert!(am.iter().any(|x| x.code == "saint_etienne" && x.date == d(2027, 12, 26)));
        let re = feries(2026, Territoire::depuis_departement(Some("974")), false);
        assert!(re.iter().any(|x| x.code == "abolition_reunion" && x.date == d(2026, 12, 20)));
        assert_eq!(feries(2026, Territoire::Metropole, false).len(), 11);
        assert!(est_alsace_moselle(None, Some("67")));
        assert!(est_alsace_moselle(Some("alsace_moselle"), None));
        assert!(!est_alsace_moselle(None, Some("75")));
    }

    #[test]
    fn annee_scolaire_chevauche_deux_annees_civiles() {
        let v = feries_entre(d(2026, 9, 1), d(2027, 8, 31), Territoire::Metropole, false);
        assert_eq!(v.len(), 11 - 0); // 1er janvier..25 décembre : chaque férié une seule fois
        assert_eq!(v.first().map(|f| f.date), Some(d(2026, 11, 1)));
        assert_eq!(v.last().map(|f| f.date), Some(d(2027, 8, 15)));
    }

    #[test]
    fn ponts_mardi_et_jeudi() {
        let f = feries(2026, Territoire::Metropole, false);
        let p = ponts_possibles(&f);
        // 14 juillet 2026 = mardi → pont le lundi 13 ; Ascension 14 mai 2026 = jeudi → vendredi 15.
        assert!(p.contains(&(d(2026, 7, 14), d(2026, 7, 13))));
        assert!(p.contains(&(d(2026, 5, 14), d(2026, 5, 15))));
    }
}
