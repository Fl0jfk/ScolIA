//! Incidents & sanctions — Vie scolaire (tables CONFIGURABLE).
//!
//! Saisie VS + consultation sur fiche élève. Motifs préfaits = templates
//! optionnels. Commission + permis à points : module `commission`.

use crate::DomainError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncidentType {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SanctionType {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SanctionMotif {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Debug, Clone)]
pub struct CreerReferentielVs {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Incident {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub description: Option<String>,
    pub created_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DeclarerIncident {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub description: Option<String>,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctionStatut {
    Active,
    Executee,
    Annulee,
}

impl SanctionStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Executee => "executee",
            Self::Annulee => "annulee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "executee" => Some(Self::Executee),
            "annulee" => Some(Self::Annulee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sanction {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub motif_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub jour: NaiveDate,
    pub duree_minutes: Option<i32>,
    pub note: Option<String>,
    pub statut: SanctionStatut,
    pub created_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct PrononcerSanction {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub motif_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub jour: NaiveDate,
    pub duree_minutes: Option<i32>,
    pub note: Option<String>,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AnnulerSanction {
    pub etablissement_id: Uuid,
    pub sanction_id: Uuid,
    pub actor_user_id: Uuid,
}

fn plan_referentiel_code_libelle(
    cmd: &CreerReferentielVs,
) -> Result<(String, String), DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_lowercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.len() < 2 || libelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(DomainError::EmptyAttributValeur);
    }
    Ok((code, libelle))
}

pub fn plan_creer_incident_type(cmd: CreerReferentielVs) -> Result<IncidentType, DomainError> {
    let (code, libelle) = plan_referentiel_code_libelle(&cmd)?;
    Ok(IncidentType {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        actif: true,
    })
}

pub fn plan_creer_sanction_type(cmd: CreerReferentielVs) -> Result<SanctionType, DomainError> {
    let (code, libelle) = plan_referentiel_code_libelle(&cmd)?;
    Ok(SanctionType {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        actif: true,
    })
}

pub fn plan_creer_sanction_motif(cmd: CreerReferentielVs) -> Result<SanctionMotif, DomainError> {
    let (code, libelle) = plan_referentiel_code_libelle(&cmd)?;
    Ok(SanctionMotif {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        actif: true,
    })
}

pub fn plan_declarer_incident(cmd: DeclarerIncident) -> Result<Incident, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    Ok(Incident {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        type_id: cmd.type_id,
        jour: cmd.jour,
        description: cmd
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        created_by_user_id: Some(cmd.actor_user_id),
    })
}

pub fn plan_prononcer_sanction(cmd: PrononcerSanction) -> Result<Sanction, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if let Some(d) = cmd.duree_minutes {
        if d <= 0 {
            return Err(DomainError::SanctionDureeInvalide);
        }
    }
    Ok(Sanction {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        type_id: cmd.type_id,
        motif_id: cmd.motif_id,
        incident_id: cmd.incident_id,
        jour: cmd.jour,
        duree_minutes: cmd.duree_minutes,
        note: cmd
            .note
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty()),
        statut: SanctionStatut::Active,
        created_by_user_id: Some(cmd.actor_user_id),
    })
}

pub fn plan_annuler_sanction(
    current: &Sanction,
    cmd: AnnulerSanction,
) -> Result<Sanction, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.sanction_id {
        return Err(DomainError::NotFound);
    }
    if current.statut == SanctionStatut::Annulee {
        return Err(DomainError::SanctionDejaAnnulee);
    }
    let mut s = current.clone();
    s.statut = SanctionStatut::Annulee;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retenue_avec_duree() {
        let s = plan_prononcer_sanction(PrononcerSanction {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            type_id: Uuid::new_v4(),
            motif_id: None,
            incident_id: None,
            jour: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            duree_minutes: Some(60),
            note: Some("devoirs non faits".into()),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(s.statut, SanctionStatut::Active);
        assert_eq!(s.duree_minutes, Some(60));
    }

    #[test]
    fn duree_negative_refusee() {
        assert_eq!(
            plan_prononcer_sanction(PrononcerSanction {
                etablissement_id: Uuid::new_v4(),
                eleve_id: Uuid::new_v4(),
                type_id: Uuid::new_v4(),
                motif_id: None,
                incident_id: None,
                jour: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
                duree_minutes: Some(0),
                note: None,
                actor_user_id: Uuid::new_v4(),
            })
            .unwrap_err(),
            DomainError::SanctionDureeInvalide
        );
    }

    #[test]
    fn annulation_idempotente_refusee() {
        let s = plan_prononcer_sanction(PrononcerSanction {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            type_id: Uuid::new_v4(),
            motif_id: None,
            incident_id: None,
            jour: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            duree_minutes: None,
            note: None,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let ann = plan_annuler_sanction(
            &s,
            AnnulerSanction {
                etablissement_id: s.etablissement_id,
                sanction_id: s.id,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(ann.statut, SanctionStatut::Annulee);
        assert!(plan_annuler_sanction(
            &ann,
            AnnulerSanction {
                etablissement_id: ann.etablissement_id,
                sanction_id: ann.id,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .is_err());
    }
}
