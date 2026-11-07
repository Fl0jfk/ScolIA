//! Cahier de texte — travail pour une classe (face élève / parent).
//!
//! Distinct des devoirs notés (`notes::Devoir`) : ici consigne pédagogique
//! sans barème, lisible par la classe et les parents liés.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CahierTexteEntree {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    pub cree_le: NaiveDate,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerCahierTexte {
    pub etablissement_id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    pub cree_le: NaiveDate,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_cahier_texte(cmd: CreerCahierTexte) -> Result<CahierTexteEntree, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let classe = cmd.classe.trim().to_string();
    let matiere = cmd.matiere_libelle.trim().to_string();
    let titre = cmd.titre.trim().to_string();
    let consigne = cmd.consigne.trim().to_string();
    if classe.is_empty() || classe.len() > 32 {
        return Err(DomainError::CahierTexteInvalide);
    }
    if matiere.is_empty() || matiere.len() > 120 {
        return Err(DomainError::CahierTexteInvalide);
    }
    if titre.is_empty() || titre.len() > 200 {
        return Err(DomainError::CahierTexteInvalide);
    }
    if consigne.is_empty() || consigne.len() > 8000 {
        return Err(DomainError::CahierTexteInvalide);
    }
    Ok(CahierTexteEntree {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        classe,
        matiere_libelle: matiere,
        titre,
        consigne,
        pour_le: cmd.pour_le,
        cree_le: cmd.cree_le,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_vide() {
        let err = plan_creer_cahier_texte(CreerCahierTexte {
            etablissement_id: Uuid::new_v4(),
            classe: "  ".into(),
            matiere_libelle: "Maths".into(),
            titre: "Ex".into(),
            consigne: "faire".into(),
            pour_le: NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
            cree_le: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::CahierTexteInvalide);
    }

    #[test]
    fn ok_minimal() {
        let e = plan_creer_cahier_texte(CreerCahierTexte {
            etablissement_id: Uuid::new_v4(),
            classe: "4B".into(),
            matiere_libelle: "Potions".into(),
            titre: "Exercices ch.3".into(),
            consigne: "Pages 12–14".into(),
            pour_le: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            cree_le: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(e.classe, "4B");
    }
}
