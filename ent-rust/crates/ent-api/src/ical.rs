//! Export iCalendar (RFC 5545) pour emplois du temps ENT.

use chrono::{DateTime, NaiveDate, NaiveTime, Timelike, Utc};
use ent_db::EdtCreneau;
use uuid::Uuid;

/// Occurrence datée d'un créneau hebdomadaire.
#[derive(Debug, Clone)]
pub struct EdtOccurrence {
    pub date: NaiveDate,
    pub creneau: EdtCreneau,
}

pub fn ical_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\r', "")
        .replace('\n', "\\n")
}

fn fold_line(line: &str) -> String {
    let bytes = line.as_bytes();
    if bytes.len() <= 75 {
        return line.to_string();
    }
    let mut out = String::new();
    let mut start = 0usize;
    while start < bytes.len() {
        let mut end = (start + 75).min(bytes.len());
        while end > start && !line.is_char_boundary(end) {
            end -= 1;
        }
        if !out.is_empty() {
            out.push('\r');
            out.push('\n');
            out.push(' ');
        }
        out.push_str(&line[start..end]);
        start = end;
    }
    out
}

fn push_folded(buf: &mut String, line: &str) {
    buf.push_str(&fold_line(line));
    buf.push_str("\r\n");
}

fn format_dtstamp(now: DateTime<Utc>) -> String {
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

fn format_dt_paris(date: NaiveDate, time: NaiveTime) -> String {
    format!(
        "{}{:02}{:02}{:02}",
        date.format("%Y%m%d"),
        time.hour(),
        time.minute(),
        time.second()
    )
}

fn event_uid(etablissement_id: Uuid, creneau_id: Uuid, date: NaiveDate) -> String {
    format!(
        "{}-{}-{}@scolia-edt",
        creneau_id,
        date.format("%Y%m%d"),
        etablissement_id
    )
}

fn event_summary(creneau: &EdtCreneau) -> String {
    if creneau.classe.is_empty() {
        creneau.matiere_libelle.clone()
    } else {
        format!("{} ({})", creneau.matiere_libelle, creneau.classe)
    }
}

/// Calendrier VCALENDAR complet (UTF-8, TZ Europe/Paris sur les DTSTART/DTEND).
pub fn render_edt_calendar(calendar_name: &str, occurrences: &[EdtOccurrence]) -> String {
    let now = Utc::now();
    let mut buf = String::with_capacity(occurrences.len() * 256 + 512);
    push_folded(&mut buf, "BEGIN:VCALENDAR");
    push_folded(&mut buf, "VERSION:2.0");
    push_folded(
        &mut buf,
        "PRODID:-//ScolIA//ENT Emploi du temps//FR",
    );
    push_folded(&mut buf, "CALSCALE:GREGORIAN");
    push_folded(&mut buf, "METHOD:PUBLISH");
    push_folded(&mut buf, &format!("X-WR-CALNAME:{}", ical_escape(calendar_name)));
    push_folded(&mut buf, "BEGIN:VTIMEZONE");
    push_folded(&mut buf, "TZID:Europe/Paris");
    push_folded(&mut buf, "BEGIN:DAYLIGHT");
    push_folded(&mut buf, "TZOFFSETFROM:+0100");
    push_folded(&mut buf, "TZOFFSETTO:+0200");
    push_folded(&mut buf, "TZNAME:CEST");
    push_folded(&mut buf, "DTSTART:19700329T020000");
    push_folded(
        &mut buf,
        "RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU",
    );
    push_folded(&mut buf, "END:DAYLIGHT");
    push_folded(&mut buf, "BEGIN:STANDARD");
    push_folded(&mut buf, "TZOFFSETFROM:+0200");
    push_folded(&mut buf, "TZOFFSETTO:+0100");
    push_folded(&mut buf, "TZNAME:CET");
    push_folded(&mut buf, "DTSTART:19701025T030000");
    push_folded(
        &mut buf,
        "RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU",
    );
    push_folded(&mut buf, "END:STANDARD");
    push_folded(&mut buf, "END:VTIMEZONE");

    for occ in occurrences {
        let c = &occ.creneau;
        let summary = ical_escape(&event_summary(c));
        let location = c
            .salle
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(ical_escape)
            .unwrap_or_default();
        let uid = event_uid(c.etablissement_id, c.id, occ.date);
        let dtstart = format_dt_paris(occ.date, c.heure_debut);
        let dtend = format_dt_paris(occ.date, c.heure_fin);
        let stamp = format_dtstamp(now);

        push_folded(&mut buf, "BEGIN:VEVENT");
        push_folded(&mut buf, &format!("UID:{}", uid));
        push_folded(&mut buf, &format!("DTSTAMP:{}", stamp));
        push_folded(
            &mut buf,
            &format!("DTSTART;TZID=Europe/Paris:{}", dtstart),
        );
        push_folded(
            &mut buf,
            &format!("DTEND;TZID=Europe/Paris:{}", dtend),
        );
        push_folded(&mut buf, &format!("SUMMARY:{}", summary));
        if !location.is_empty() {
            push_folded(&mut buf, &format!("LOCATION:{}", location));
        }
        if !c.site.trim().is_empty() {
            push_folded(
                &mut buf,
                &format!("DESCRIPTION:Site {}", ical_escape(&c.site)),
            );
        }
        push_folded(&mut buf, "END:VEVENT");
    }

    push_folded(&mut buf, "END:VCALENDAR");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_eleve::EdtSemaineAb;

    fn sample_creneau() -> EdtCreneau {
        EdtCreneau {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            classe: "4B".into(),
            matiere_id: None,
            matiere_libelle: "Mathématiques".into(),
            prof_user_id: Uuid::new_v4(),
            jour_semaine: 1,
            heure_debut: NaiveTime::from_hms_opt(8, 30, 0).unwrap(),
            heure_fin: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            salle: Some("S12".into()),
            site: String::new(),
            semaine_ab: EdtSemaineAb::Toutes,
        }
    }

    #[test]
    fn render_contains_vevent_and_summary() {
        let c = sample_creneau();
        let date = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        let ics = render_edt_calendar(
            "Test",
            &[EdtOccurrence {
                date,
                creneau: c,
            }],
        );
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("SUMMARY:Math"));
        assert!(ics.contains("TZID=Europe/Paris"));
        assert!(ics.contains("LOCATION:S12"));
    }

    #[test]
    fn escape_semicolon() {
        assert_eq!(ical_escape("a;b"), "a\\;b");
    }
}
