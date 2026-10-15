//! Stages / entreprises / conventions (P2).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entreprise {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerEntreprise {
    pub etablissement_id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_entreprise(cmd: CreerEntreprise) -> Result<Entreprise, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let raison = cmd.raison_sociale.trim().to_string();
    if raison.is_empty() || raison.len() > 200 {
        return Err(DomainError::StageInvalide);
    }
    let adresse = cmd.adresse.trim().to_string();
    if adresse.len() > 400 {
        return Err(DomainError::StageInvalide);
    }
    let siret = cmd
        .siret
        .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
        .filter(|s| !s.is_empty());
    if siret.as_ref().is_some_and(|s| s.len() != 14) {
        return Err(DomainError::StageInvalide);
    }
    Ok(Entreprise {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        raison_sociale: raison,
        siret,
        adresse,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConventionStage {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub entreprise_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
    pub statut: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerConventionStage {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub entreprise_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_convention_stage(
    cmd: CreerConventionStage,
) -> Result<ConventionStage, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() || cmd.entreprise_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::StageInvalide);
    }
    let tuteur = cmd.tuteur_nom.trim().to_string();
    if tuteur.is_empty() || tuteur.len() > 120 {
        return Err(DomainError::StageInvalide);
    }
    Ok(ConventionStage {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        entreprise_id: cmd.entreprise_id,
        debut: cmd.debut,
        fin: cmd.fin,
        tuteur_nom: tuteur,
        statut: "active".into(),
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siret_14() {
        let err = plan_creer_entreprise(CreerEntreprise {
            etablissement_id: Uuid::new_v4(),
            raison_sociale: "ACME".into(),
            siret: Some("123".into()),
            adresse: "1 rue".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::StageInvalide);
    }

    #[test]
    fn convention_dates() {
        let err = plan_creer_convention_stage(CreerConventionStage {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            entreprise_id: Uuid::new_v4(),
            debut: NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            tuteur_nom: "Dupont".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::StageInvalide);
    }
}
