//! Retards — fait de vie scolaire **canonique**, distinct de l'absence.
//!
//! Un retard est saisi à l'appel (`LigneEtat::Retard`) ou posé à la main par la
//! vie scolaire. Il vit dans `ent_retard` : il n'alimente **pas** le compteur
//! d'absences (un retard n'est pas une absence) mais il est **persistant**
//! (historique élève, compteurs, lecture famille) au lieu de disparaître à la
//! clôture de l'appel.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Retard {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    /// Durée du retard en minutes (facultatif).
    pub minutes: Option<i32>,
    /// Note libre (« bus », « rendez-vous médical »…), facultatif.
    pub motif: Option<String>,
    /// Appel d'origine (si le retard vient de l'appel) — support d'idempotence.
    pub source_appel_id: Option<Uuid>,
    /// Auteur (staff) du geste, si connu.
    pub auteur_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeclarerRetard {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub minutes: Option<i32>,
    pub motif: Option<String>,
    pub source_appel_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
}

/// Une durée de retard au-delà de 10 h est une saisie erronée, pas un retard.
const MAX_MINUTES: i32 = 600;

pub fn plan_declarer_retard(cmd: DeclarerRetard) -> Result<Retard, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::TenantMismatch);
    }
    if let Some(m) = cmd.minutes {
        if !(0..=MAX_MINUTES).contains(&m) {
            return Err(DomainError::RetardInvalide);
        }
    }
    let motif = cmd
        .motif
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    if let Some(m) = &motif {
        if m.len() > 200 {
            return Err(DomainError::RetardInvalide);
        }
    }
    Ok(Retard {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        minutes: cmd.minutes,
        motif,
        source_appel_id: cmd.source_appel_id,
        auteur_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd() -> DeclarerRetard {
        DeclarerRetard {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            minutes: Some(15),
            motif: Some("Bus".into()),
            source_appel_id: None,
            actor_user_id: None,
        }
    }

    #[test]
    fn declare_un_retard_valide() {
        let r = plan_declarer_retard(cmd()).unwrap();
        assert_eq!(r.minutes, Some(15));
        assert_eq!(r.motif.as_deref(), Some("Bus"));
    }

    #[test]
    fn duree_aberrante_refusee() {
        let mut c = cmd();
        c.minutes = Some(-1);
        assert!(plan_declarer_retard(c).is_err());
        let mut c = cmd();
        c.minutes = Some(9999);
        assert!(plan_declarer_retard(c).is_err());
    }

    #[test]
    fn motif_vide_devient_none_et_etab_nil_refuse() {
        let mut c = cmd();
        c.motif = Some("   ".into());
        assert_eq!(plan_declarer_retard(c).unwrap().motif, None);
        let mut c = cmd();
        c.etablissement_id = Uuid::nil();
        assert!(plan_declarer_retard(c).is_err());
    }
}
