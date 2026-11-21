//! Absences personnel / RH (P1) — signal jour pour remplacements.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbsenceRh {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeclarerAbsenceRh {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub actor_user_id: Uuid,
}

pub fn plan_declarer_absence_rh(cmd: DeclarerAbsenceRh) -> Result<AbsenceRh, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.user_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::AbsenceRhInvalide);
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() || motif.len() > 200 {
        return Err(DomainError::AbsenceRhInvalide);
    }
    Ok(AbsenceRh {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        user_id: cmd.user_id,
        debut: cmd.debut,
        fin: cmd.fin,
        motif,
        statut: "signalee".into(),
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_valider_absence_rh(
    current: AbsenceRh,
    actor_user_id: Uuid,
) -> Result<AbsenceRh, DomainError> {
    if current.statut != "signalee" {
        return Err(DomainError::AbsenceRhInvalide);
    }
    Ok(AbsenceRh {
        statut: "validee".into(),
        created_by_user_id: actor_user_id,
        ..current
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        let err = plan_declarer_absence_rh(DeclarerAbsenceRh {
            etablissement_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            debut: NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            motif: "maladie".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::AbsenceRhInvalide);
    }
}
