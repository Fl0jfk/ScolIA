//! Paramètres établissement (P1) — prefs métier typées librement.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParamEtab {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub cle: String,
    pub valeur: String,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertParamEtab {
    pub etablissement_id: Uuid,
    pub cle: String,
    pub valeur: String,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_param_etab(cmd: UpsertParamEtab) -> Result<ParamEtab, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let cle = cmd.cle.trim().to_ascii_lowercase();
    if cle.is_empty() || cle.len() > 64 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
        return Err(DomainError::ParamEtabInvalide);
    }
    let valeur = cmd.valeur.trim().to_string();
    if valeur.len() > 4000 {
        return Err(DomainError::ParamEtabInvalide);
    }
    Ok(ParamEtab {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        cle,
        valeur,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cle_normalisee() {
        let p = plan_upsert_param_etab(UpsertParamEtab {
            etablissement_id: Uuid::new_v4(),
            cle: " Seuil_Absences ".into(),
            valeur: "5".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(p.cle, "seuil_absences");
    }
}
