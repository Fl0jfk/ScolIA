//! Fiche santé élève (P0) — une fiche par élève, upsert infirmière.
//!
//! Famille : allergies / traitements / contre-indications / PAI / médecin / urgence.
//! Note confidentielle jamais exposée au parent.
//! Pas d'ordonnances PDF ni PAI interactif ici.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FicheSante {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub allergies: String,
    pub traitements: String,
    pub contre_indications: String,
    pub pai_actif: bool,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
    /// Note clinique — jamais exposée à la famille.
    pub note_confidentielle: String,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertFicheSante {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub allergies: String,
    pub traitements: String,
    pub contre_indications: String,
    pub pai_actif: bool,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
    pub note_confidentielle: String,
    pub actor_user_id: Uuid,
}

/// Vue famille : note confidentielle vidée.
pub fn vue_famille(f: &FicheSante) -> FicheSante {
    FicheSante {
        note_confidentielle: String::new(),
        ..f.clone()
    }
}

fn trim_opt(s: Option<String>, max: usize) -> Result<Option<String>, DomainError> {
    match s {
        None => Ok(None),
        Some(raw) => {
            let t = raw.trim().to_string();
            if t.is_empty() {
                Ok(None)
            } else if t.len() > max {
                Err(DomainError::FicheSanteInvalide)
            } else {
                Ok(Some(t))
            }
        }
    }
}

fn trim_field(s: String, max: usize) -> Result<String, DomainError> {
    let t = s.trim().to_string();
    if t.len() > max {
        return Err(DomainError::FicheSanteInvalide);
    }
    Ok(t)
}

pub fn plan_upsert_fiche_sante(cmd: UpsertFicheSante) -> Result<FicheSante, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let allergies = trim_field(cmd.allergies, 2000)?;
    let traitements = trim_field(cmd.traitements, 2000)?;
    let contre_indications = trim_field(cmd.contre_indications, 2000)?;
    let note = trim_field(cmd.note_confidentielle, 4000)?;
    // Au moins un champ utile (sinon fiche vide inutile).
    if allergies.is_empty()
        && traitements.is_empty()
        && contre_indications.is_empty()
        && note.is_empty()
        && !cmd.pai_actif
        && cmd.medecin_nom.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
        && cmd.urgence_tel.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
    {
        return Err(DomainError::FicheSanteInvalide);
    }
    Ok(FicheSante {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        allergies,
        traitements,
        contre_indications,
        pai_actif: cmd.pai_actif,
        medecin_nom: trim_opt(cmd.medecin_nom, 200)?,
        urgence_tel: trim_opt(cmd.urgence_tel, 40)?,
        note_confidentielle: note,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_fiche_vide() {
        let err = plan_upsert_fiche_sante(UpsertFicheSante {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            allergies: "  ".into(),
            traitements: String::new(),
            contre_indications: String::new(),
            pai_actif: false,
            medecin_nom: None,
            urgence_tel: None,
            note_confidentielle: String::new(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::FicheSanteInvalide);
    }

    #[test]
    fn vue_famille_masque_note() {
        let f = plan_upsert_fiche_sante(UpsertFicheSante {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            allergies: "arachides".into(),
            traitements: "Ventoline".into(),
            contre_indications: String::new(),
            pai_actif: true,
            medecin_nom: Some("Dr Pomfrey".into()),
            urgence_tel: Some("0600000000".into()),
            note_confidentielle: "antécédent asthme sévère".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let v = vue_famille(&f);
        assert!(v.note_confidentielle.is_empty());
        assert_eq!(v.allergies, "arachides");
        assert!(v.pai_actif);
    }
}
