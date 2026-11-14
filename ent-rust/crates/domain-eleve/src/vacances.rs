//! Vacances / jours fermés établissement (P1).
//!
//! Périodes de vacances + jours fériés / fermeture — consommés par EDT / cantine.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VacancesKind {
    Vacances,
    Ferie,
    Fermeture,
}

impl VacancesKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vacances => "vacances",
            Self::Ferie => "ferie",
            Self::Fermeture => "fermeture",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "vacances" | "vacance" => Some(Self::Vacances),
            "ferie" | "férié" | "feriee" => Some(Self::Ferie),
            "fermeture" | "ferme" => Some(Self::Fermeture),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeriodeVacances {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub libelle: String,
    pub kind: VacancesKind,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl PeriodeVacances {
    pub fn couvre(&self, jour: NaiveDate) -> bool {
        self.debut <= jour && jour <= self.fin
    }
}

#[derive(Debug, Clone)]
pub struct CreerPeriodeVacances {
    pub etablissement_id: Uuid,
    pub libelle: String,
    pub kind: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_periode_vacances(
    cmd: CreerPeriodeVacances,
) -> Result<PeriodeVacances, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::VacancesInvalide);
    }
    let libelle = cmd.libelle.trim().to_string();
    if libelle.is_empty() || libelle.len() > 200 {
        return Err(DomainError::VacancesInvalide);
    }
    let kind = VacancesKind::parse(&cmd.kind).ok_or(DomainError::VacancesInvalide)?;
    Ok(PeriodeVacances {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        libelle,
        kind,
        debut: cmd.debut,
        fin: cmd.fin,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn jour_est_ferme(periodes: &[PeriodeVacances], jour: NaiveDate) -> bool {
    periodes.iter().any(|p| p.couvre(jour))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toussaint_couvre() {
        let p = plan_creer_periode_vacances(CreerPeriodeVacances {
            etablissement_id: Uuid::new_v4(),
            libelle: "Toussaint".into(),
            kind: "vacances".into(),
            debut: NaiveDate::from_ymd_opt(2026, 10, 17).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert!(p.couvre(NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()));
        assert!(!p.couvre(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()));
    }
}
