//! Journal des modifications métier (P1) — audit trail par élève / agrégat.
//!
//! Chaque mutation significative peut publier une entrée lisible staff
//! (et optionnellement famille). Pas un dump SQL : libellé métier + payload.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEntree {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub domaine: String,
    pub action: String,
    pub libelle: String,
    pub payload: Value,
    pub visible_famille: bool,
    pub actor_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EnregistrerJournal {
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub domaine: String,
    pub action: String,
    pub libelle: String,
    pub payload: Value,
    pub visible_famille: bool,
    pub actor_user_id: Option<Uuid>,
}

pub fn plan_enregistrer_journal(cmd: EnregistrerJournal) -> Result<JournalEntree, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let domaine = cmd.domaine.trim().to_string();
    let action = cmd.action.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if domaine.is_empty() || action.is_empty() || libelle.is_empty() {
        return Err(DomainError::JournalInvalide);
    }
    if domaine.len() > 64 || action.len() > 64 || libelle.len() > 500 {
        return Err(DomainError::JournalInvalide);
    }
    Ok(JournalEntree {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        domaine,
        action,
        libelle,
        payload: cmd.payload,
        visible_famille: cmd.visible_famille,
        actor_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn refuse_libellle_vide() {
        let err = plan_enregistrer_journal(EnregistrerJournal {
            etablissement_id: Uuid::new_v4(),
            eleve_id: None,
            domaine: "vs".into(),
            action: "dispense".into(),
            libelle: "  ".into(),
            payload: json!({}),
            visible_famille: false,
            actor_user_id: None,
        })
        .unwrap_err();
        assert_eq!(err, DomainError::JournalInvalide);
    }
}
