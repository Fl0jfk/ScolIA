//! Event → actions d'écriture **additionnelles** (pas de re-copie du socle).
//!
//! Create / change classe / change statut : déjà sur `ent_eleve`.
//! Clôture d'appel : absents → `EnsureAbsence` sur la table canonique `ent_absence`
//! (une vérité pour VS, cantine, famille).

use crate::actions::BrainAction;
use ent_events::{types, MetierEvent};

pub fn react(event: &MetierEvent) -> Vec<BrainAction> {
    if event.type_name != types::APPEL_CLOSED {
        return Vec::new();
    }

    let Some(absents) = event.payload.get("absents").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let jour = event
        .payload
        .get("jour")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if jour.is_empty() {
        return Vec::new();
    }

    absents
        .iter()
        .filter_map(|v| v.as_str())
        .filter_map(|s| uuid::Uuid::parse_str(s).ok())
        .map(|eleve_id| BrainAction::EnsureAbsence {
            etablissement_id: event.etablissement_id,
            eleve_id,
            motif: "appel".into(),
            debut: jour.clone(),
            fin: jour.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_events::{AggregateKind, MetierEvent};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn appel_closed_produit_ensure_absence() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let event = MetierEvent {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            type_name: types::APPEL_CLOSED.into(),
            aggregate: AggregateKind::Appel,
            aggregate_id: Uuid::new_v4(),
            eleve_id: None,
            payload: json!({
                "jour": "2026-10-12",
                "classe": "4B",
                "absents": [eleve.to_string()],
            }),
            actor_user_id: None,
            created_at: chrono::Utc::now(),
        };
        let actions = react(&event);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            BrainAction::EnsureAbsence {
                eleve_id,
                motif,
                debut,
                ..
            } => {
                assert_eq!(*eleve_id, eleve);
                assert_eq!(motif, "appel");
                assert_eq!(debut, "2026-10-12");
            }
        }
    }

    #[test]
    fn autres_events_aucune_action() {
        let event = MetierEvent {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            type_name: types::ELEVE_CREATED.into(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: Uuid::new_v4(),
            eleve_id: None,
            payload: json!({}),
            actor_user_id: None,
            created_at: chrono::Utc::now(),
        };
        assert!(react(&event).is_empty());
    }
}
