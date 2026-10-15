//! Carnet de liaison / observations — Vie scolaire (P0).
//!
//! Entrée staff → visible famille (optionnel) → notif messagerie → accusé parent (signature).
//! Notif famille : messagerie + SMTP (branchement dans ent-db / ent-mail).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarnetCategorie {
    Correspondance,
    Accompagnement,
    Information,
}

impl CarnetCategorie {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correspondance => "correspondance",
            Self::Accompagnement => "accompagnement",
            Self::Information => "information",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "correspondance" => Some(Self::Correspondance),
            "accompagnement" => Some(Self::Accompagnement),
            "information" => Some(Self::Information),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarnetEntree {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_entree: NaiveDate,
    pub categorie: CarnetCategorie,
    pub titre: String,
    pub corps: String,
    pub visible_famille: bool,
    pub created_by_user_id: Option<Uuid>,
    pub signe_at: Option<DateTime<Utc>>,
    pub signe_par_user_id: Option<Uuid>,
    /// Fil messagerie ouvert si `visible_famille` (notif parent).
    pub fil_notif_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct CreerCarnetEntree {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_entree: NaiveDate,
    pub categorie: CarnetCategorie,
    pub titre: String,
    pub corps: String,
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct SignerCarnetEntree {
    pub etablissement_id: Uuid,
    pub entree_id: Uuid,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_carnet_entree(cmd: CreerCarnetEntree) -> Result<CarnetEntree, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let titre = cmd.titre.trim().to_string();
    let corps = cmd.corps.trim().to_string();
    if titre.is_empty() || corps.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    Ok(CarnetEntree {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        date_entree: cmd.date_entree,
        categorie: cmd.categorie,
        titre,
        corps,
        visible_famille: cmd.visible_famille,
        created_by_user_id: Some(cmd.actor_user_id),
        signe_at: None,
        signe_par_user_id: None,
        fil_notif_id: None,
    })
}

/// Sujet du fil de notif carnet.
pub fn sujet_notif_carnet(titre: &str) -> String {
    format!("Carnet de liaison — {titre}")
}

/// Corps du message de notif carnet.
pub fn corps_notif_carnet(categorie: &str, titre: &str, corps: &str) -> String {
    format!(
        "Une nouvelle entrée a été ajoutée au carnet de liaison ({categorie}).\n\n\
         {titre}\n\n{corps}\n\n\
         Merci de la consulter et de la signer dans le portail famille."
    )
}

pub fn plan_signer_carnet_entree(
    entree: &CarnetEntree,
    cmd: SignerCarnetEntree,
) -> Result<CarnetEntree, DomainError> {
    if entree.etablissement_id != cmd.etablissement_id || entree.id != cmd.entree_id {
        return Err(DomainError::TenantMismatch);
    }
    if !entree.visible_famille {
        return Err(DomainError::CarnetNonVisibleFamille);
    }
    if entree.signe_at.is_some() {
        return Err(DomainError::CarnetDejaSigne);
    }
    Ok(CarnetEntree {
        signe_at: Some(Utc::now()),
        signe_par_user_id: Some(cmd.actor_user_id),
        ..entree.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creer_refuse_titre_vide() {
        let err = plan_creer_carnet_entree(CreerCarnetEntree {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            date_entree: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            categorie: CarnetCategorie::Information,
            titre: "  ".into(),
            corps: "ok".into(),
            visible_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EmptyAttributValeur);
    }

    #[test]
    fn signer_deux_fois_refuse() {
        let e = plan_creer_carnet_entree(CreerCarnetEntree {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            date_entree: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            categorie: CarnetCategorie::Correspondance,
            titre: "Mot".into(),
            corps: "Travail à revoir".into(),
            visible_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let parent = Uuid::new_v4();
        let signed = plan_signer_carnet_entree(
            &e,
            SignerCarnetEntree {
                etablissement_id: e.etablissement_id,
                entree_id: e.id,
                actor_user_id: parent,
            },
        )
        .unwrap();
        let err = plan_signer_carnet_entree(
            &signed,
            SignerCarnetEntree {
                etablissement_id: e.etablissement_id,
                entree_id: e.id,
                actor_user_id: parent,
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::CarnetDejaSigne);
    }
}
