//! Infos complémentaires paramétrables (ADM_IC*) — P2.
//!
//! Définition de champs par établissement + valeurs par élève.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoCompType {
    Texte,
    Nombre,
    Booleen,
    Date,
}

impl InfoCompType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Texte => "texte",
            Self::Nombre => "nombre",
            Self::Booleen => "booleen",
            Self::Date => "date",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "texte" | "text" | "string" => Some(Self::Texte),
            "nombre" | "number" | "int" => Some(Self::Nombre),
            "booleen" | "bool" | "boolean" => Some(Self::Booleen),
            "date" => Some(Self::Date),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfoCompChamp {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub type_valeur: InfoCompType,
    pub obligatoire: bool,
    pub visible_famille: bool,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerInfoCompChamp {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub type_valeur: String,
    pub obligatoire: bool,
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_info_comp_champ(cmd: CreerInfoCompChamp) -> Result<InfoCompChamp, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || libelle.is_empty() || code.len() > 64 || libelle.len() > 200 {
        return Err(DomainError::InfoCompInvalide);
    }
    let type_valeur = InfoCompType::parse(&cmd.type_valeur).ok_or(DomainError::InfoCompInvalide)?;
    Ok(InfoCompChamp {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        type_valeur,
        obligatoire: cmd.obligatoire,
        visible_famille: cmd.visible_famille,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfoCompValeur {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub champ_id: Uuid,
    pub valeur: String,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertInfoCompValeur {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub champ_id: Uuid,
    pub valeur: String,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_info_comp_valeur(
    cmd: UpsertInfoCompValeur,
    champ: &InfoCompChamp,
) -> Result<InfoCompValeur, DomainError> {
    if champ.etablissement_id != cmd.etablissement_id || champ.id != cmd.champ_id {
        return Err(DomainError::InfoCompInvalide);
    }
    if !champ.actif {
        return Err(DomainError::InfoCompInvalide);
    }
    let valeur = cmd.valeur.trim().to_string();
    if champ.obligatoire && valeur.is_empty() {
        return Err(DomainError::InfoCompInvalide);
    }
    if valeur.len() > 2000 {
        return Err(DomainError::InfoCompInvalide);
    }
    match champ.type_valeur {
        InfoCompType::Nombre if !valeur.is_empty() && valeur.parse::<f64>().is_err() => {
            return Err(DomainError::InfoCompInvalide);
        }
        InfoCompType::Booleen if !valeur.is_empty() => {
            let ok = matches!(
                valeur.to_ascii_lowercase().as_str(),
                "true" | "false" | "1" | "0" | "oui" | "non"
            );
            if !ok {
                return Err(DomainError::InfoCompInvalide);
            }
        }
        InfoCompType::Date if !valeur.is_empty() => {
            if chrono::NaiveDate::parse_from_str(&valeur, "%Y-%m-%d").is_err() {
                return Err(DomainError::InfoCompInvalide);
            }
        }
        _ => {}
    }
    Ok(InfoCompValeur {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        champ_id: cmd.champ_id,
        valeur,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_invalide_refuse() {
        let champ = plan_creer_info_comp_champ(CreerInfoCompChamp {
            etablissement_id: Uuid::new_v4(),
            code: "nb_freres".into(),
            libelle: "Nb frères".into(),
            type_valeur: "nombre".into(),
            obligatoire: false,
            visible_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let err = plan_upsert_info_comp_valeur(
            UpsertInfoCompValeur {
                etablissement_id: champ.etablissement_id,
                eleve_id: Uuid::new_v4(),
                champ_id: champ.id,
                valeur: "abc".into(),
                actor_user_id: Uuid::new_v4(),
            },
            &champ,
        )
        .unwrap_err();
        assert_eq!(err, DomainError::InfoCompInvalide);
    }
}
