//! Taux / catégories TVA établissement (P2) — référentiel compta / OGEC.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TauxTva {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    /// Taux en basis points (ex. 550 = 5,5 %, 2000 = 20 %).
    pub taux_bps: i32,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerTauxTva {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub taux_bps: i32,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_taux_tva(cmd: CreerTauxTva) -> Result<TauxTva, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || code.len() > 32 || libelle.is_empty() || libelle.len() > 120 {
        return Err(DomainError::TvaInvalide);
    }
    if !(0..=10000).contains(&cmd.taux_bps) {
        return Err(DomainError::TvaInvalide);
    }
    Ok(TauxTva {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        taux_bps: cmd.taux_bps,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

/// HT → TTC (centimes) pour un taux en basis points.
pub fn ttc_depuis_ht(ht_centimes: i64, taux_bps: i32) -> i64 {
    if ht_centimes <= 0 || taux_bps <= 0 {
        return ht_centimes.max(0);
    }
    let tva = (ht_centimes.saturating_mul(i64::from(taux_bps))) / 10_000;
    ht_centimes.saturating_add(tva)
}

/// TTC → HT (centimes) — arrondi bancaire simple.
pub fn ht_depuis_ttc(ttc_centimes: i64, taux_bps: i32) -> i64 {
    if ttc_centimes <= 0 {
        return 0;
    }
    if taux_bps <= 0 {
        return ttc_centimes;
    }
    (ttc_centimes.saturating_mul(10_000)) / (10_000 + i64::from(taux_bps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vingt_pourcent() {
        assert_eq!(ttc_depuis_ht(10000, 2000), 12000);
        assert_eq!(ht_depuis_ttc(12000, 2000), 10000);
        let err = plan_creer_taux_tva(CreerTauxTva {
            etablissement_id: Uuid::new_v4(),
            code: "".into(),
            libelle: "X".into(),
            taux_bps: 2000,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::TvaInvalide);
    }
}
