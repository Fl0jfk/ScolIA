//! Infirmerie — passages (P0).
//!
//! Note de soins confidentielle ; famille ne voit que motif + jour si `visible_famille`.
//! Fiche santé structurée : module `fiche_sante`.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassageInfirmerieMotif {
    Soins,
    Malaise,
    Accident,
    PaiSuivi,
    Autre,
}

impl PassageInfirmerieMotif {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soins => "soins",
            Self::Malaise => "malaise",
            Self::Accident => "accident",
            Self::PaiSuivi => "pai_suivi",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "soins" => Some(Self::Soins),
            "malaise" => Some(Self::Malaise),
            "accident" => Some(Self::Accident),
            "pai_suivi" => Some(Self::PaiSuivi),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Soins => "Soins",
            Self::Malaise => "Malaise",
            Self::Accident => "Accident",
            Self::PaiSuivi => "Suivi PAI",
            Self::Autre => "Autre",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassageInfirmerie {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub motif: PassageInfirmerieMotif,
    /// Note clinique — jamais exposée à la famille.
    pub note_soins: String,
    pub visible_famille: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerPassageInfirmerie {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub motif: PassageInfirmerieMotif,
    pub note_soins: String,
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

/// Vue famille : motif + jour, note vide.
pub fn vue_famille(p: &PassageInfirmerie) -> PassageInfirmerie {
    PassageInfirmerie {
        note_soins: String::new(),
        ..p.clone()
    }
}

pub fn plan_creer_passage_infirmerie(
    cmd: CreerPassageInfirmerie,
) -> Result<PassageInfirmerie, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let note = cmd.note_soins.trim().to_string();
    if note.is_empty() || note.len() > 4000 {
        return Err(DomainError::PassageInfirmerieInvalide);
    }
    Ok(PassageInfirmerie {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        motif: cmd.motif,
        note_soins: note,
        visible_famille: cmd.visible_famille,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_note_vide() {
        let err = plan_creer_passage_infirmerie(CreerPassageInfirmerie {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            motif: PassageInfirmerieMotif::Malaise,
            note_soins: "  ".into(),
            visible_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::PassageInfirmerieInvalide);
    }

    #[test]
    fn vue_famille_masque_note() {
        let p = plan_creer_passage_infirmerie(CreerPassageInfirmerie {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            motif: PassageInfirmerieMotif::Soins,
            note_soins: "pansement doigt".into(),
            visible_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let v = vue_famille(&p);
        assert!(v.note_soins.is_empty());
        assert_eq!(v.motif, PassageInfirmerieMotif::Soins);
    }
}
