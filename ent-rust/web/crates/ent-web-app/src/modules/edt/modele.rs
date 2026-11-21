//! Logique de l'emploi du temps (pure, testée) : semaines et libellés.

use chrono::{Datelike, Duration, NaiveDate};

use crate::noyau::temps::nom_mois;

/// Lundi de la semaine de `jour`.
pub fn lundi_de(jour: NaiveDate) -> NaiveDate {
    jour.checked_sub_signed(Duration::days(i64::from(
        jour.weekday().num_days_from_monday(),
    )))
    .unwrap_or(jour)
}

/// Jours ouvrés (lundi → vendredi) de la semaine commençant `lundi`.
pub fn jours_ouvres(lundi: NaiveDate) -> [NaiveDate; 5] {
    std::array::from_fn(|i| {
        lundi
            .checked_add_signed(Duration::days(i as i64))
            .unwrap_or(lundi)
    })
}

/// « 1er », « 2 »… (le premier du mois s’écrit « 1er »).
fn quantieme(d: NaiveDate) -> String {
    if d.day() == 1 {
        "1er".into()
    } else {
        d.day().to_string()
    }
}

/// « Semaine du 2 au 6 novembre », « Semaine du 29 décembre au 2 janvier ».
pub fn libelle_semaine(lundi: NaiveDate) -> String {
    let vendredi = lundi.checked_add_signed(Duration::days(4)).unwrap_or(lundi);
    if lundi.month() == vendredi.month() {
        format!(
            "Semaine du {} au {} {}",
            quantieme(lundi),
            quantieme(vendredi),
            nom_mois(vendredi.month())
        )
    } else {
        format!(
            "Semaine du {} {} au {} {}",
            quantieme(lundi),
            nom_mois(lundi.month()),
            quantieme(vendredi),
            nom_mois(vendredi.month())
        )
    }
}

/// Années acceptées dans `?semaine=` (une adresse forgée ne doit jamais faire paniquer).
pub const ANNEES_ACCEPTEES: std::ops::RangeInclusive<i32> = 2000..=2100;

/// Lundi demandé dans l'adresse (`?semaine=AAAA-MM-JJ`, ramené au lundi) ; paramètre absent,
/// illisible ou hors de [`ANNEES_ACCEPTEES`] → semaine courante.
pub fn semaine_demandee(parametre: Option<String>, aujourdhui: NaiveDate) -> NaiveDate {
    let jour = parametre
        .and_then(|p| p.parse::<NaiveDate>().ok())
        .filter(|d| ANNEES_ACCEPTEES.contains(&d.year()))
        .unwrap_or(aujourdhui);
    lundi_de(jour)
}

/// Lundi de la semaine décalée de `decalage` semaines ; `None` si on sortirait des années acceptées.
pub fn semaine_voisine(lundi: NaiveDate, decalage: i64) -> Option<NaiveDate> {
    let d = lundi.checked_add_signed(Duration::try_weeks(decalage)?)?;
    ANNEES_ACCEPTEES.contains(&d.year()).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn semaines() {
        assert_eq!(lundi_de(d(2026, 11, 3)), d(2026, 11, 2), "mardi → lundi");
        assert_eq!(
            lundi_de(d(2026, 11, 8)),
            d(2026, 11, 2),
            "dimanche → lundi précédent"
        );
        assert_eq!(lundi_de(d(2026, 11, 2)), d(2026, 11, 2));
        let j = jours_ouvres(d(2026, 11, 2));
        assert_eq!(j[4], d(2026, 11, 6));
        assert_eq!(
            libelle_semaine(d(2026, 11, 2)),
            "Semaine du 2 au 6 novembre"
        );
        assert_eq!(
            libelle_semaine(d(2026, 12, 28)),
            "Semaine du 28 décembre au 1er janvier"
        );
        assert_eq!(
            semaine_demandee(Some("2026-11-05".into()), d(2026, 1, 1)),
            d(2026, 11, 2)
        );
        assert_eq!(
            semaine_demandee(Some("n'importe quoi".into()), d(2026, 11, 3)),
            d(2026, 11, 2)
        );
        assert_eq!(semaine_demandee(None, d(2026, 11, 3)), d(2026, 11, 2));
    }

    #[test]
    fn semaine_forgee_ne_panique_pas() {
        let auj = d(2026, 11, 3);
        let lundi_auj = d(2026, 11, 2);
        for p in [
            "262142-12-31",
            "+262142-12-31",
            "-262143-01-01",
            "9999-12-31",
            "1999-12-31",
            "2101-01-01",
            "0000-01-01",
        ] {
            assert_eq!(semaine_demandee(Some(p.into()), auj), lundi_auj, "{p}");
        }
        assert_eq!(
            semaine_demandee(Some("2100-12-31".into()), auj),
            d(2100, 12, 27)
        );
        assert_eq!(
            semaine_demandee(Some("2000-01-01".into()), auj),
            d(1999, 12, 27)
        );
        // Fonctions de base sûres aux bornes de chrono.
        let max = NaiveDate::MAX;
        let _ = (lundi_de(max), jours_ouvres(max), libelle_semaine(max));
        let min = NaiveDate::MIN;
        let _ = (lundi_de(min), jours_ouvres(min), libelle_semaine(min));
    }

    #[test]
    fn semaine_voisine_bornee() {
        assert_eq!(semaine_voisine(d(2026, 11, 2), 1), Some(d(2026, 11, 9)));
        assert_eq!(semaine_voisine(d(2026, 11, 2), -1), Some(d(2026, 10, 26)));
        assert_eq!(semaine_voisine(d(2100, 12, 27), 1), None);
        assert_eq!(semaine_voisine(d(2026, 11, 2), i64::MAX), None);
    }
}
