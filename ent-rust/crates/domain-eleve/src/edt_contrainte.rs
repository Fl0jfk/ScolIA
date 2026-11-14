//! Contraintes EDT soft (préférences / interdits) — pas un solveur CP-SAT.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdtContrainteKind {
    /// Jour + plage à éviter pour un prof.
    ProfIndispo,
    /// Salle indisponible (code) sur un créneau hebdo.
    SalleIndispo,
    /// Préférence matière / classe (soft).
    Preference,
}

impl EdtContrainteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfIndispo => "prof_indispo",
            Self::SalleIndispo => "salle_indispo",
            Self::Preference => "preference",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "prof_indispo" => Some(Self::ProfIndispo),
            "salle_indispo" => Some(Self::SalleIndispo),
            "preference" => Some(Self::Preference),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdtContrainte {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub kind: EdtContrainteKind,
    pub prof_user_id: Option<Uuid>,
    pub salle_code: Option<String>,
    /// ISO 1=lundi … 7=dimanche ; None = tous.
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<chrono::NaiveTime>,
    pub heure_fin: Option<chrono::NaiveTime>,
    pub detail: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerEdtContrainte {
    pub etablissement_id: Uuid,
    pub kind: EdtContrainteKind,
    pub prof_user_id: Option<Uuid>,
    pub salle_code: Option<String>,
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<chrono::NaiveTime>,
    pub heure_fin: Option<chrono::NaiveTime>,
    pub detail: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_edt_contrainte(cmd: CreerEdtContrainte) -> Result<EdtContrainte, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let detail = cmd.detail.trim().to_string();
    if detail.len() > 500 {
        return Err(DomainError::EdtContrainteInvalide);
    }
    if let Some(js) = cmd.jour_semaine {
        if !(1..=7).contains(&js) {
            return Err(DomainError::EdtContrainteInvalide);
        }
    }
    if let (Some(d), Some(f)) = (cmd.heure_debut, cmd.heure_fin) {
        if f <= d {
            return Err(DomainError::EdtContrainteInvalide);
        }
    }
    match cmd.kind {
        EdtContrainteKind::ProfIndispo => {
            if cmd.prof_user_id.filter(|u| !u.is_nil()).is_none() {
                return Err(DomainError::EdtContrainteInvalide);
            }
        }
        EdtContrainteKind::SalleIndispo => {
            let s = cmd.salle_code.as_deref().unwrap_or("").trim();
            if s.is_empty() || s.len() > 80 {
                return Err(DomainError::EdtContrainteInvalide);
            }
        }
        EdtContrainteKind::Preference => {}
    }
    let salle = cmd
        .salle_code
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(EdtContrainte {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        kind: cmd.kind,
        prof_user_id: cmd.prof_user_id.filter(|u| !u.is_nil()),
        salle_code: salle,
        jour_semaine: cmd.jour_semaine,
        heure_debut: cmd.heure_debut,
        heure_fin: cmd.heure_fin,
        detail,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prof_indispo_exige_prof() {
        let err = plan_creer_edt_contrainte(CreerEdtContrainte {
            etablissement_id: Uuid::new_v4(),
            kind: EdtContrainteKind::ProfIndispo,
            prof_user_id: None,
            salle_code: None,
            jour_semaine: Some(1),
            heure_debut: None,
            heure_fin: None,
            detail: "réunion".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EdtContrainteInvalide);
    }
}
