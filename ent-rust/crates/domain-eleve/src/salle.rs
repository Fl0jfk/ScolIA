//! Salles / locaux établissement (P0/P2) — référentiel EDT.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Salle {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub capacite: Option<i32>,
    pub site: Option<String>,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerSalle {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub capacite: Option<i32>,
    pub site: Option<String>,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_salle(cmd: CreerSalle) -> Result<Salle, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || code.len() > 32 || libelle.is_empty() || libelle.len() > 120 {
        return Err(DomainError::SalleInvalide);
    }
    if let Some(c) = cmd.capacite {
        if c < 0 || c > 500 {
            return Err(DomainError::SalleInvalide);
        }
    }
    let site = cmd
        .site
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if site.as_ref().is_some_and(|s| s.len() > 64) {
        return Err(DomainError::SalleInvalide);
    }
    Ok(Salle {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        capacite: cmd.capacite,
        site,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_upper() {
        let s = plan_creer_salle(CreerSalle {
            etablissement_id: Uuid::new_v4(),
            code: "a12".into(),
            libelle: "Salle A12".into(),
            capacite: Some(30),
            site: Some("principal".into()),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(s.code, "A12");
    }
}
