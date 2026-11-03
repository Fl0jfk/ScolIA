//! Dates et heures en français (sans dépendance de localisation).

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};

/// Date du jour (heure locale du navigateur ; heure du serveur pendant le rendu).
pub fn aujourdhui() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Heure locale courante.
pub fn maintenant() -> NaiveTime {
    chrono::Local::now().time()
}

pub fn nom_jour(j: Weekday) -> &'static str {
    match j {
        Weekday::Mon => "lundi",
        Weekday::Tue => "mardi",
        Weekday::Wed => "mercredi",
        Weekday::Thu => "jeudi",
        Weekday::Fri => "vendredi",
        Weekday::Sat => "samedi",
        Weekday::Sun => "dimanche",
    }
}

pub fn nom_mois(m: u32) -> &'static str {
    [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ]
    .get(m.saturating_sub(1) as usize)
    .copied()
    .unwrap_or("")
}

fn majuscule(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// « Mardi 3 novembre ».
pub fn date_longue(d: NaiveDate) -> String {
    majuscule(&format!(
        "{} {} {}",
        nom_jour(d.weekday()),
        if d.day() == 1 {
            "1er".to_string()
        } else {
            d.day().to_string()
        },
        nom_mois(d.month())
    ))
}

/// « Mardi 3 novembre 2026 ».
pub fn date_complete(d: NaiveDate) -> String {
    format!("{} {}", date_longue(d), d.year())
}

/// « 08:00 » à partir de « 08:00 » ou « 08:00:00 ».
pub fn heure_courte(h: &str) -> &str {
    h.get(..5).unwrap_or(h)
}

/// Heure lisible d'une chaîne « HH:MM ».
pub fn lire_heure(h: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(heure_courte(h), "%H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_en_francais() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        assert_eq!(date_longue(d), "Mardi 3 novembre");
        assert_eq!(date_complete(d), "Mardi 3 novembre 2026");
        assert_eq!(heure_courte("08:00:00"), "08:00");
        assert_eq!(lire_heure("13:30"), NaiveTime::from_hms_opt(13, 30, 0));
        assert_eq!(nom_mois(13), "");
    }
}
