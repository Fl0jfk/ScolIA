//! Tarifs consommation cantine / formules (P2).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TarifConso {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerTarifConso {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_tarif_conso(cmd: CreerTarifConso) -> Result<TarifConso, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || code.len() > 32 || libelle.is_empty() || libelle.len() > 120 {
        return Err(DomainError::TarifConsoInvalide);
    }
    if cmd.montant_centimes < 0 {
        return Err(DomainError::TarifConsoInvalide);
    }
    if cmd.fin.is_some_and(|f| f < cmd.debut) {
        return Err(DomainError::TarifConsoInvalide);
    }
    Ok(TarifConso {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        montant_centimes: cmd.montant_centimes,
        debut: cmd.debut,
        fin: cmd.fin,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn tarif_couvre(t: &TarifConso, jour: NaiveDate) -> bool {
    t.actif && t.debut <= jour && t.fin.map(|f| jour <= f).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montant_negatif() {
        let err = plan_creer_tarif_conso(CreerTarifConso {
            etablissement_id: Uuid::new_v4(),
            code: "MIDI".into(),
            libelle: "Repas midi".into(),
            montant_centimes: -1,
            debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            fin: None,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::TarifConsoInvalide);
    }
}
