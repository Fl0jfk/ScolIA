//! Quotient familial foyer (P2) — base de calcul bourses / tarifs.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuotientFoyer {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub annee_scolaire: String,
    /// Quotient en centimes d'euro (ex. 85050 = 850,50 €).
    pub quotient_centimes: i64,
    /// Nombre de parts fiscales (×100, ex. 250 = 2,5).
    pub parts_x100: i32,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertQuotientFoyer {
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub annee_scolaire: String,
    pub quotient_centimes: i64,
    pub parts_x100: i32,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_quotient_foyer(cmd: UpsertQuotientFoyer) -> Result<QuotientFoyer, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.foyer_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let annee = cmd.annee_scolaire.trim().to_string();
    if annee.is_empty() || annee.len() > 20 {
        return Err(DomainError::QuotientInvalide);
    }
    if cmd.quotient_centimes < 0 || cmd.parts_x100 <= 0 || cmd.parts_x100 > 2000 {
        return Err(DomainError::QuotientInvalide);
    }
    Ok(QuotientFoyer {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        foyer_id: cmd.foyer_id,
        annee_scolaire: annee,
        quotient_centimes: cmd.quotient_centimes,
        parts_x100: cmd.parts_x100,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_positives() {
        let err = plan_upsert_quotient_foyer(UpsertQuotientFoyer {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            annee_scolaire: "2026-2027".into(),
            quotient_centimes: 10000,
            parts_x100: 0,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::QuotientInvalide);
    }
}
