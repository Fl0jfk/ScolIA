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

// ---------------------------------------------------------------------------
// Heure de Paris (les créneaux de rendez-vous sont stockés en UTC)
// ---------------------------------------------------------------------------

use chrono::{DateTime, Duration, NaiveDateTime, Utc};

fn dernier_dimanche(annee: i32, mois: u32) -> NaiveDate {
    let suivant = if mois == 12 {
        NaiveDate::from_ymd_opt(annee + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(annee, mois + 1, 1)
    }
    .expect("date");
    let mut d = suivant - Duration::days(1);
    while d.weekday() != Weekday::Sun {
        d -= Duration::days(1);
    }
    d
}

/// Heure d'été en France : du dernier dimanche de mars 01:00 UTC au dernier dimanche d'octobre 01:00 UTC.
fn heure_d_ete(utc: NaiveDateTime) -> bool {
    let annee = utc.year();
    let debut = dernier_dimanche(annee, 3)
        .and_hms_opt(1, 0, 0)
        .expect("heure");
    let fin = dernier_dimanche(annee, 10)
        .and_hms_opt(1, 0, 0)
        .expect("heure");
    utc >= debut && utc < fin
}

/// Instant UTC → heure locale de Paris.
pub fn a_paris(utc: DateTime<Utc>) -> NaiveDateTime {
    let n = utc.naive_utc();
    n + Duration::hours(if heure_d_ete(n) { 2 } else { 1 })
}

/// Heure locale de Paris → instant UTC (heure d'été retenue si elle s'applique).
pub fn paris_vers_utc(local: NaiveDateTime) -> DateTime<Utc> {
    let ete = local - Duration::hours(2);
    let utc = if heure_d_ete(ete) {
        ete
    } else {
        local - Duration::hours(1)
    };
    DateTime::from_naive_utc_and_offset(utc, Utc)
}

/// « Mardi 3 novembre, 09:10 » (heure de Paris).
pub fn date_heure_paris(utc: DateTime<Utc>) -> String {
    let l = a_paris(utc);
    format!("{}, {}", date_longue(l.date()), l.format("%H:%M"))
}

#[cfg(test)]
mod tests_paris {
    use super::*;

    fn utc(a: i32, m: u32, j: u32, h: u32, mi: u32) -> DateTime<Utc> {
        DateTime::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(a, m, j)
                .unwrap()
                .and_hms_opt(h, mi, 0)
                .unwrap(),
            Utc,
        )
    }

    #[test]
    fn hiver_et_ete() {
        assert_eq!(
            a_paris(utc(2026, 11, 6, 8, 0)).format("%H:%M").to_string(),
            "09:00"
        );
        assert_eq!(
            a_paris(utc(2026, 6, 15, 8, 0)).format("%H:%M").to_string(),
            "10:00"
        );
    }

    #[test]
    fn bascules_de_l_heure() {
        // 2026 : passage à l'heure d'été le 29 mars, retour le 25 octobre (01:00 UTC).
        assert_eq!(
            a_paris(utc(2026, 3, 29, 0, 59)).format("%H:%M").to_string(),
            "01:59"
        );
        assert_eq!(
            a_paris(utc(2026, 3, 29, 1, 0)).format("%H:%M").to_string(),
            "03:00"
        );
        assert_eq!(
            a_paris(utc(2026, 10, 25, 0, 59))
                .format("%H:%M")
                .to_string(),
            "02:59"
        );
        assert_eq!(
            a_paris(utc(2026, 10, 25, 1, 0)).format("%H:%M").to_string(),
            "02:00"
        );
    }

    #[test]
    fn aller_retour_hors_bascule() {
        for (m, j, h) in [(1, 15, 9), (7, 4, 18), (11, 6, 7), (4, 2, 12)] {
            let u = utc(2026, m, j, h, 30);
            assert_eq!(paris_vers_utc(a_paris(u)), u);
        }
    }
}
