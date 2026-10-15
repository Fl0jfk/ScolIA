//! Menus cantine hebdomadaires (P2) — publication famille.
//!
//! Une entrée par jour : entrée / plat / dessert (+ allergenotes libres).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuCantineJour {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub jour: NaiveDate,
    pub entree: String,
    pub plat: String,
    pub dessert: String,
    pub notes: String,
    pub publie: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertMenuCantine {
    pub etablissement_id: Uuid,
    pub jour: NaiveDate,
    pub entree: String,
    pub plat: String,
    pub dessert: String,
    pub notes: String,
    pub publie: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_menu_cantine(cmd: UpsertMenuCantine) -> Result<MenuCantineJour, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let plat = cmd.plat.trim().to_string();
    if plat.is_empty() {
        return Err(DomainError::MenuCantineInvalide);
    }
    let entree = cmd.entree.trim().to_string();
    let dessert = cmd.dessert.trim().to_string();
    let notes = cmd.notes.trim().to_string();
    if entree.len() > 500 || plat.len() > 500 || dessert.len() > 500 || notes.len() > 1000 {
        return Err(DomainError::MenuCantineInvalide);
    }
    Ok(MenuCantineJour {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        jour: cmd.jour,
        entree,
        plat,
        dessert,
        notes,
        publie: cmd.publie,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plat_obligatoire() {
        let err = plan_upsert_menu_cantine(UpsertMenuCantine {
            etablissement_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            entree: "Salade".into(),
            plat: "  ".into(),
            dessert: "Yaourt".into(),
            notes: String::new(),
            publie: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::MenuCantineInvalide);
    }
}
