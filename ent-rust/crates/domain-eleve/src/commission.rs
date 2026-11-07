//! Commission de vie scolaire + permis à points (P0).
//!
//! Session → convocation élève → décision. Points retirés à la décision
//! (solde = POINTS_VS_DEPART − somme des retraits décidés).
//! Famille : objet + motifs + décision si `signale_famille` — jamais note interne.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Solde initial permis à points (par élève, dérivé).
pub const POINTS_VS_DEPART: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommissionStatut {
    Planifiee,
    Tenue,
    Annulee,
}

impl CommissionStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planifiee => "planifiee",
            Self::Tenue => "tenue",
            Self::Annulee => "annulee",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Planifiee => "Planifiée",
            Self::Tenue => "Tenue",
            Self::Annulee => "Annulée",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "planifiee" => Some(Self::Planifiee),
            "tenue" => Some(Self::Tenue),
            "annulee" => Some(Self::Annulee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCommission {
    Avertissement,
    ExclusionTemporaire,
    Maintien,
    Autre,
}

impl DecisionCommission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Avertissement => "avertissement",
            Self::ExclusionTemporaire => "exclusion_temporaire",
            Self::Maintien => "maintien",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "avertissement" => Some(Self::Avertissement),
            "exclusion_temporaire" => Some(Self::ExclusionTemporaire),
            "maintien" => Some(Self::Maintien),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Avertissement => "Avertissement",
            Self::ExclusionTemporaire => "Exclusion temporaire",
            Self::Maintien => "Maintien",
            Self::Autre => "Autre",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommissionVs {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub objet: String,
    pub lieu: String,
    pub statut: CommissionStatut,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerCommissionVs {
    pub etablissement_id: Uuid,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub objet: String,
    pub lieu: String,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommissionDossier {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub commission_id: Uuid,
    pub eleve_id: Uuid,
    pub motifs: String,
    pub note_interne: String,
    pub sanction_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub decision: Option<DecisionCommission>,
    pub decision_texte: String,
    pub points_retires: i32,
    pub signale_famille: bool,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by_user_id: Option<Uuid>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConvoquerCommission {
    pub etablissement_id: Uuid,
    pub commission_id: Uuid,
    pub eleve_id: Uuid,
    pub motifs: String,
    pub note_interne: String,
    pub sanction_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub signale_famille: bool,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct DeciderCommissionDossier {
    pub etablissement_id: Uuid,
    pub dossier_id: Uuid,
    pub decision: DecisionCommission,
    pub decision_texte: String,
    pub points_retires: i32,
    pub actor_user_id: Uuid,
}

/// Vue famille : note interne vidée.
pub fn vue_dossier_famille(d: &CommissionDossier) -> CommissionDossier {
    CommissionDossier {
        note_interne: String::new(),
        ..d.clone()
    }
}

/// Solde permis à points dérivé.
pub fn solde_points_vs(points_retires_sum: i32) -> i32 {
    POINTS_VS_DEPART.saturating_sub(points_retires_sum.max(0))
}

pub fn plan_creer_commission(cmd: CreerCommissionVs) -> Result<CommissionVs, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let objet = cmd.objet.trim().to_string();
    let lieu = cmd.lieu.trim().to_string();
    if objet.is_empty() || objet.len() > 500 {
        return Err(DomainError::CommissionVsInvalide);
    }
    if lieu.len() > 200 {
        return Err(DomainError::CommissionVsInvalide);
    }
    Ok(CommissionVs {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        jour: cmd.jour,
        heure: cmd.heure,
        objet,
        lieu,
        statut: CommissionStatut::Planifiee,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_convoquer_commission(
    commission: &CommissionVs,
    cmd: ConvoquerCommission,
) -> Result<CommissionDossier, DomainError> {
    if commission.etablissement_id != cmd.etablissement_id
        || commission.id != cmd.commission_id
    {
        return Err(DomainError::NotFound);
    }
    if commission.statut == CommissionStatut::Annulee {
        return Err(DomainError::CommissionVsInvalide);
    }
    if cmd.eleve_id.is_nil() {
        return Err(DomainError::CommissionVsInvalide);
    }
    let motifs = cmd.motifs.trim().to_string();
    if motifs.is_empty() || motifs.len() > 4000 {
        return Err(DomainError::CommissionVsInvalide);
    }
    let note = cmd.note_interne.trim().to_string();
    if note.len() > 4000 {
        return Err(DomainError::CommissionVsInvalide);
    }
    Ok(CommissionDossier {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        commission_id: cmd.commission_id,
        eleve_id: cmd.eleve_id,
        motifs,
        note_interne: note,
        sanction_id: cmd.sanction_id,
        incident_id: cmd.incident_id,
        decision: None,
        decision_texte: String::new(),
        points_retires: 0,
        signale_famille: cmd.signale_famille,
        decided_at: None,
        decided_by_user_id: None,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_decider_dossier(
    current: &CommissionDossier,
    commission: &CommissionVs,
    cmd: &DeciderCommissionDossier,
) -> Result<(CommissionDossier, CommissionVs), DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.dossier_id {
        return Err(DomainError::NotFound);
    }
    if current.decision.is_some() {
        return Err(DomainError::CommissionDejaDecidee);
    }
    if commission.statut == CommissionStatut::Annulee {
        return Err(DomainError::CommissionVsInvalide);
    }
    if cmd.points_retires < 0 || cmd.points_retires > POINTS_VS_DEPART {
        return Err(DomainError::CommissionVsInvalide);
    }
    let texte = cmd.decision_texte.trim().to_string();
    if texte.len() > 4000 {
        return Err(DomainError::CommissionVsInvalide);
    }
    let dossier = CommissionDossier {
        decision: Some(cmd.decision),
        decision_texte: texte,
        points_retires: cmd.points_retires,
        decided_at: Some(Utc::now()),
        decided_by_user_id: Some(cmd.actor_user_id),
        ..current.clone()
    };
    let commission = CommissionVs {
        statut: CommissionStatut::Tenue,
        ..commission.clone()
    };
    Ok((dossier, commission))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_commission_points() {
        let c = plan_creer_commission(CreerCommissionVs {
            etablissement_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            objet: "Incidents répétés".into(),
            lieu: "Bureau CPE".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let d = plan_convoquer_commission(
            &c,
            ConvoquerCommission {
                etablissement_id: c.etablissement_id,
                commission_id: c.id,
                eleve_id: Uuid::new_v4(),
                motifs: "Perturbations cumulées".into(),
                note_interne: "dossier confidentiel".into(),
                sanction_id: None,
                incident_id: None,
                signale_famille: true,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        let v = vue_dossier_famille(&d);
        assert!(v.note_interne.is_empty());
        let (decided, tenue) = plan_decider_dossier(
            &d,
            &c,
            &DeciderCommissionDossier {
                etablissement_id: c.etablissement_id,
                dossier_id: d.id,
                decision: DecisionCommission::Avertissement,
                decision_texte: "Avertissement écrit".into(),
                points_retires: 5,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(tenue.statut, CommissionStatut::Tenue);
        assert_eq!(decided.points_retires, 5);
        assert_eq!(solde_points_vs(5), 15);
        let err = plan_decider_dossier(
            &decided,
            &tenue,
            &DeciderCommissionDossier {
                etablissement_id: c.etablissement_id,
                dossier_id: d.id,
                decision: DecisionCommission::Maintien,
                decision_texte: String::new(),
                points_retires: 0,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::CommissionDejaDecidee);
    }
}
