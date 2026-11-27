//! Horloge et jour métier de l'établissement.
//!
//! **Règle** : « aujourd'hui » n'est jamais la date UTC. Entre 23 h et 0 h UTC (1 h/2 h du matin
//! à Paris) la date UTC et la date de l'établissement diffèrent ; un jour calculé en UTC fait
//! alors diverger le code Rust de la base (`ent_aujourdhui_etab`, fuseau du profil
//! d'établissement, S2) et des vues (`ent_eleve_lien_actif`…).
//!
//! * Côté base : `ent_aujourdhui_etab(etab[, instant])` est la source (fuseau de l'établissement).
//! * Côté Rust, quand la connexion/transaction n'est pas à portée : [`jour_paris`] (fuseau par
//!   défaut `Europe/Paris`, règle UE de l'heure d'été, sans dépendance de fuseaux).
//! * Pour les tests : [`Horloge`] / [`Fixe`] et les fonctions pures [`date_paris`] /
//!   [`jour_paris_a`] qui prennent l'instant en paramètre (scénario « 23 h 30 UTC »).
//!
//! Limite connue : [`jour_paris`] ignore le fuseau propre d'un établissement hors métropole ;
//! les chemins qui connaissent l'établissement doivent préférer `ent_aujourdhui_etab`.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc, Weekday};

/// Source du temps, injectable dans les tests.
pub trait Horloge: Send + Sync {
    fn maintenant(&self) -> DateTime<Utc>;
}

/// Horloge système.
#[derive(Debug, Clone, Copy, Default)]
pub struct Systeme;

impl Horloge for Systeme {
    fn maintenant(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Horloge figée à un instant (tests).
#[derive(Debug, Clone, Copy)]
pub struct Fixe(pub DateTime<Utc>);

impl Horloge for Fixe {
    fn maintenant(&self) -> DateTime<Utc> {
        self.0
    }
}

fn dernier_dimanche(annee: i32, mois: u32) -> NaiveDate {
    let dernier_jour = if mois == 12 {
        NaiveDate::from_ymd_opt(annee + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(annee, mois + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .expect("date valide");
    let mut d = dernier_jour;
    while d.weekday() != Weekday::Sun {
        d = d.pred_opt().expect("date valide");
    }
    d
}

/// Décalage de Paris par rapport à l'UTC, en heures (+1 hiver, +2 heure d'été de l'UE : du
/// dernier dimanche de mars 01:00 UTC au dernier dimanche d'octobre 01:00 UTC).
pub fn decalage_paris_heures(t: DateTime<Utc>) -> i64 {
    let a = t.year();
    let debut = Utc.from_utc_datetime(&dernier_dimanche(a, 3).and_hms_opt(1, 0, 0).expect("heure"));
    let fin = Utc.from_utc_datetime(&dernier_dimanche(a, 10).and_hms_opt(1, 0, 0).expect("heure"));
    if t >= debut && t < fin {
        2
    } else {
        1
    }
}

/// Date civile à Paris d'un instant UTC.
pub fn date_paris(t: DateTime<Utc>) -> NaiveDate {
    (t + Duration::hours(decalage_paris_heures(t))).date_naive()
}

/// Jour métier (Paris) selon l'horloge fournie.
pub fn jour_paris_a(h: &dyn Horloge) -> NaiveDate {
    date_paris(h.maintenant())
}

/// Jour métier courant (Paris), horloge système.
pub fn jour_paris() -> NaiveDate {
    date_paris(Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).expect("rfc3339").with_timezone(&Utc)
    }

    #[test]
    fn minuit_paris_hiver_et_ete() {
        // Hiver (UTC+1) : 23 h 30 UTC = 0 h 30 le lendemain à Paris.
        assert_eq!(date_paris(utc("2026-11-10T23:30:00Z")), NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        assert_eq!(date_paris(utc("2026-11-10T22:59:59Z")), NaiveDate::from_ymd_opt(2026, 11, 10).unwrap());
        // Été (UTC+2) : la bascule est à 22 h UTC.
        assert_eq!(date_paris(utc("2026-07-14T22:00:00Z")), NaiveDate::from_ymd_opt(2026, 7, 15).unwrap());
        assert_eq!(date_paris(utc("2026-07-14T21:59:59Z")), NaiveDate::from_ymd_opt(2026, 7, 14).unwrap());
    }

    #[test]
    fn bascules_heure_d_ete() {
        // 2026 : passage à l'été le 29 mars à 01:00 UTC, retour à l'heure d'hiver le 25 octobre.
        assert_eq!(decalage_paris_heures(utc("2026-03-29T00:59:59Z")), 1);
        assert_eq!(decalage_paris_heures(utc("2026-03-29T01:00:00Z")), 2);
        assert_eq!(decalage_paris_heures(utc("2026-10-25T00:59:59Z")), 2);
        assert_eq!(decalage_paris_heures(utc("2026-10-25T01:00:00Z")), 1);
        // Changement d'année : 31 décembre 23 h UTC = 1er janvier à Paris.
        assert_eq!(date_paris(utc("2026-12-31T23:00:00Z")), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    }

    #[test]
    fn horloge_fixe_injectable() {
        let h = Fixe(utc("2026-11-10T23:30:00Z"));
        assert_eq!(h.maintenant().date_naive(), NaiveDate::from_ymd_opt(2026, 11, 10).unwrap());
        assert_eq!(jour_paris_a(&h), NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
    }

    /// Scénario du constat CI : à 23 h 30 UTC, « le retour est demain » (UTC) est en réalité
    /// aujourd'hui (Paris) — le statut dérivé doit suivre le jour de l'établissement.
    #[test]
    fn scenario_23h30_utc_le_retour_de_demain_utc_est_aujourd_hui_a_paris() {
        use crate::{statut_derive, PeriodeScolarisation, StatutEleve};
        let h = Fixe(utc("2026-11-10T23:30:00Z"));
        let jour_utc = h.maintenant().date_naive();
        let jour_etab = jour_paris_a(&h);
        assert_eq!(jour_etab, jour_utc + Duration::days(1));
        // Sorti le jour (UTC), réintégré « demain » (UTC) = jour de l'établissement.
        let periodes = [
            PeriodeScolarisation {
                date_entree: jour_utc - Duration::days(100),
                date_sortie: Some(jour_utc),
                motif_sortie: None,
            },
            PeriodeScolarisation { date_entree: jour_utc + Duration::days(1), date_sortie: None, motif_sortie: None },
        ];
        assert_eq!(statut_derive(&periodes, jour_etab), StatutEleve::Scolarise);
        // Un test qui raisonnerait en UTC attendrait « a_venir » : c'était le défaut des tests.
        assert_eq!(statut_derive(&periodes, jour_utc), StatutEleve::AVenir);
    }
}
