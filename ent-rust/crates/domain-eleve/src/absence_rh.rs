//! Absences personnel / RH (P1) — signal jour pour remplacements.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbsenceRh {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: String,
    /// Auteur de la DÉCLARATION (jamais écrasé par la validation, AR-13 / R-AP5). Colonne
    /// historique : `created_by_user_id` ne change plus après la création.
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    /// Qui a validé / refusé / annulé, et quand (colonnes distinctes, R-AP5).
    pub valide_par_user_id: Option<Uuid>,
    pub valide_le: Option<DateTime<Utc>>,
    pub refuse_par_user_id: Option<Uuid>,
    pub refuse_le: Option<DateTime<Utc>>,
    pub annule_par_user_id: Option<Uuid>,
    pub annule_le: Option<DateTime<Utc>>,
    /// Confirmée par l'agent lui-même ; une absence seulement signalée par un tiers ne bloque pas
    /// la personne comme remplaçante (§28.4 point 4).
    pub confirmee: bool,
}

#[derive(Debug, Clone)]
pub struct DeclarerAbsenceRh {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub actor_user_id: Uuid,
}

pub fn plan_declarer_absence_rh(cmd: DeclarerAbsenceRh) -> Result<AbsenceRh, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.user_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::AbsenceRhInvalide);
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() || motif.len() > 200 {
        return Err(DomainError::AbsenceRhInvalide);
    }
    Ok(AbsenceRh {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        user_id: cmd.user_id,
        debut: cmd.debut,
        fin: cmd.fin,
        motif,
        statut: "signalee".into(),
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
        valide_par_user_id: None,
        valide_le: None,
        refuse_par_user_id: None,
        refuse_le: None,
        annule_par_user_id: None,
        annule_le: None,
        confirmee: cmd.actor_user_id == cmd.user_id,
    })
}

/// Valide une absence `signalee`. Personne ne valide sa propre absence (AR-06, R-AP10) ; l'auteur de
/// la déclaration est conservé (AR-13) : le valideur et la date vont dans leurs propres colonnes.
pub fn plan_valider_absence_rh(
    current: AbsenceRh,
    actor_user_id: Uuid,
) -> Result<AbsenceRh, DomainError> {
    if current.statut != "signalee" {
        return Err(DomainError::AbsenceRhInvalide);
    }
    if actor_user_id == current.user_id {
        return Err(DomainError::AbsenceRhAutoValidation);
    }
    Ok(AbsenceRh {
        statut: "validee".into(),
        valide_par_user_id: Some(actor_user_id),
        valide_le: Some(Utc::now()),
        ..current
    })
}

/// Refuse une absence `signalee` (AR-16). Même règle : jamais l'absent lui-même.
pub fn plan_refuser_absence_rh(
    current: AbsenceRh,
    actor_user_id: Uuid,
) -> Result<AbsenceRh, DomainError> {
    if current.statut != "signalee" {
        return Err(DomainError::AbsenceRhInvalide);
    }
    if actor_user_id == current.user_id {
        return Err(DomainError::AbsenceRhAutoValidation);
    }
    Ok(AbsenceRh {
        statut: "refusee".into(),
        refuse_par_user_id: Some(actor_user_id),
        refuse_le: Some(Utc::now()),
        ..current
    })
}

/// Annule une absence saisie par erreur ou qui n'a pas eu lieu (AR-16) : rien n'est supprimé.
/// Possible depuis `signalee` ou `validee`.
pub fn plan_annuler_absence_rh(
    current: AbsenceRh,
    actor_user_id: Uuid,
) -> Result<AbsenceRh, DomainError> {
    if !matches!(current.statut.as_str(), "signalee" | "validee") {
        return Err(DomainError::AbsenceRhInvalide);
    }
    Ok(AbsenceRh {
        statut: "annulee".into(),
        annule_par_user_id: Some(actor_user_id),
        annule_le: Some(Utc::now()),
        ..current
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        let err = plan_declarer_absence_rh(DeclarerAbsenceRh {
            etablissement_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            debut: NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            motif: "maladie".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::AbsenceRhInvalide);
    }

    fn absence(statut: &str) -> AbsenceRh {
        let user = Uuid::new_v4();
        AbsenceRh {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            user_id: user,
            debut: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
            motif: "Absence".into(),
            statut: statut.into(),
            created_by_user_id: user,
            created_at: Utc::now(),
            valide_par_user_id: None,
            valide_le: None,
            refuse_par_user_id: None,
            refuse_le: None,
            annule_par_user_id: None,
            annule_le: None,
            confirmee: true,
        }
    }

    #[test]
    fn auto_validation_et_auto_refus_refuses() {
        let a = absence("signalee");
        let moi = a.user_id;
        assert_eq!(
            plan_valider_absence_rh(a.clone(), moi).unwrap_err(),
            DomainError::AbsenceRhAutoValidation
        );
        assert_eq!(
            plan_refuser_absence_rh(a, moi).unwrap_err(),
            DomainError::AbsenceRhAutoValidation
        );
    }

    #[test]
    fn la_validation_conserve_le_declarant() {
        let a = absence("signalee");
        let declarant = a.created_by_user_id;
        let valideur = Uuid::new_v4();
        let v = plan_valider_absence_rh(a, valideur).unwrap();
        assert_eq!(v.created_by_user_id, declarant, "AR-13");
        assert_eq!(v.valide_par_user_id, Some(valideur));
        assert!(v.valide_le.is_some());
    }

    #[test]
    fn refuser_et_annuler() {
        let r = plan_refuser_absence_rh(absence("signalee"), Uuid::new_v4()).unwrap();
        assert_eq!(r.statut, "refusee");
        assert!(r.refuse_par_user_id.is_some());
        // Une absence refusée ne se valide plus ni ne se refuse deux fois.
        assert!(plan_valider_absence_rh(r.clone(), Uuid::new_v4()).is_err());
        assert!(plan_annuler_absence_rh(r, Uuid::new_v4()).is_err());
        let v = plan_valider_absence_rh(absence("signalee"), Uuid::new_v4()).unwrap();
        let a = plan_annuler_absence_rh(v, Uuid::new_v4()).unwrap();
        assert_eq!(a.statut, "annulee");
        assert!(a.annule_le.is_some());
    }

    #[test]
    fn declarer_par_un_tiers_n_est_pas_confirmee() {
        let user = Uuid::new_v4();
        let cmd = |actor| DeclarerAbsenceRh {
            etablissement_id: Uuid::new_v4(),
            user_id: user,
            debut: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            motif: "x".into(),
            actor_user_id: actor,
        };
        assert!(plan_declarer_absence_rh(cmd(user)).unwrap().confirmee);
        assert!(!plan_declarer_absence_rh(cmd(Uuid::new_v4())).unwrap().confirmee);
    }
}
