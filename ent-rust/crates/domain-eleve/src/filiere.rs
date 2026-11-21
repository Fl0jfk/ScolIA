//! Filières / MEF / vœux inscription (P1).

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Filiere {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub mef: Option<String>,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerFiliere {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub mef: Option<String>,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_filiere(cmd: CreerFiliere) -> Result<Filiere, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || code.len() > 32 || libelle.is_empty() || libelle.len() > 160 {
        return Err(DomainError::FiliereInvalide);
    }
    let mef = cmd
        .mef
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    if mef.as_ref().is_some_and(|m| m.len() > 32) {
        return Err(DomainError::FiliereInvalide);
    }
    Ok(Filiere {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        mef,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoeuInscription {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub filiere_id: Uuid,
    pub rang: i16,
    pub annee_scolaire: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PoserVoeu {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub filiere_id: Uuid,
    pub rang: i16,
    pub annee_scolaire: String,
    pub actor_user_id: Uuid,
}

pub fn plan_poser_voeu(cmd: PoserVoeu) -> Result<VoeuInscription, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() || cmd.filiere_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if !(1..=5).contains(&cmd.rang) {
        return Err(DomainError::FiliereInvalide);
    }
    let annee = cmd.annee_scolaire.trim().to_string();
    if annee.is_empty() || annee.len() > 20 {
        return Err(DomainError::FiliereInvalide);
    }
    Ok(VoeuInscription {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        filiere_id: cmd.filiere_id,
        rang: cmd.rang,
        annee_scolaire: annee,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rang_borne() {
        let err = plan_poser_voeu(PoserVoeu {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            filiere_id: Uuid::new_v4(),
            rang: 0,
            annee_scolaire: "2026-2027".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::FiliereInvalide);
    }
}
