//! Conseil de classe — planification + dossier élève + mention (P2).
//!
//! Session par classe/période → dossiers élèves → décision/mention.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConseilClasseStatut {
    Planifie,
    Tenu,
    Annule,
}

impl ConseilClasseStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planifie => "planifie",
            Self::Tenu => "tenu",
            Self::Annule => "annule",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "planifie" | "planifié" => Some(Self::Planifie),
            "tenu" => Some(Self::Tenu),
            "annule" | "annulé" => Some(Self::Annule),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionConseil {
    Felicitations,
    Complement,
    Encouragements,
    AvertissementTravail,
    AvertissementConduite,
    Aucune,
}

impl MentionConseil {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Felicitations => "felicitations",
            Self::Complement => "complement",
            Self::Encouragements => "encouragements",
            Self::AvertissementTravail => "avertissement_travail",
            Self::AvertissementConduite => "avertissement_conduite",
            Self::Aucune => "aucune",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "felicitations" | "félicitations" => Some(Self::Felicitations),
            "complement" | "complément" | "complementaires" => Some(Self::Complement),
            "encouragements" => Some(Self::Encouragements),
            "avertissement_travail" | "avertissement travail" => Some(Self::AvertissementTravail),
            "avertissement_conduite" | "avertissement conduite" => {
                Some(Self::AvertissementConduite)
            }
            "aucune" | "sans" | "" => Some(Self::Aucune),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConseilClasse {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub classe: String,
    pub periode_libelle: String,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub lieu: String,
    pub statut: ConseilClasseStatut,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerConseilClasse {
    pub etablissement_id: Uuid,
    pub classe: String,
    pub periode_libelle: String,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub lieu: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_conseil_classe(cmd: CreerConseilClasse) -> Result<ConseilClasse, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let classe = cmd.classe.trim().to_string();
    let periode = cmd.periode_libelle.trim().to_string();
    let lieu = cmd.lieu.trim().to_string();
    if classe.is_empty() || periode.is_empty() {
        return Err(DomainError::ConseilClasseInvalide);
    }
    Ok(ConseilClasse {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        classe,
        periode_libelle: periode,
        jour: cmd.jour,
        heure: cmd.heure,
        lieu: if lieu.is_empty() {
            "Salle des conseils".into()
        } else {
            lieu
        },
        statut: ConseilClasseStatut::Planifie,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConseilClasseDossier {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub conseil_id: Uuid,
    pub eleve_id: Uuid,
    pub appreciation: String,
    pub mention: Option<MentionConseil>,
    pub decision_texte: String,
    pub visible_famille: bool,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AjouterDossierConseil {
    pub etablissement_id: Uuid,
    pub conseil_id: Uuid,
    pub eleve_id: Uuid,
    pub actor_user_id: Uuid,
}

pub fn plan_ajouter_dossier_conseil(
    cmd: AjouterDossierConseil,
) -> Result<ConseilClasseDossier, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.conseil_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    Ok(ConseilClasseDossier {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        conseil_id: cmd.conseil_id,
        eleve_id: cmd.eleve_id,
        appreciation: String::new(),
        mention: None,
        decision_texte: String::new(),
        visible_famille: false,
        decided_at: None,
        decided_by_user_id: None,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone)]
pub struct DeciderDossierConseil {
    pub etablissement_id: Uuid,
    pub dossier_id: Uuid,
    pub appreciation: String,
    pub mention: String,
    pub decision_texte: String,
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_decider_dossier_conseil(
    mut current: ConseilClasseDossier,
    cmd: DeciderDossierConseil,
) -> Result<ConseilClasseDossier, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.dossier_id {
        return Err(DomainError::NotFound);
    }
    if current.decided_at.is_some() {
        return Err(DomainError::ConseilDossierDejaDecide);
    }
    let appreciation = cmd.appreciation.trim().to_string();
    let decision_texte = cmd.decision_texte.trim().to_string();
    if appreciation.len() > 4000 || decision_texte.len() > 2000 {
        return Err(DomainError::ConseilClasseInvalide);
    }
    let mention = MentionConseil::parse(&cmd.mention).ok_or(DomainError::ConseilClasseInvalide)?;
    current.appreciation = appreciation;
    current.mention = Some(mention);
    current.decision_texte = decision_texte;
    current.visible_famille = cmd.visible_famille;
    current.decided_at = Some(Utc::now());
    current.decided_by_user_id = Some(cmd.actor_user_id);
    Ok(current)
}

pub fn plan_tenir_conseil(
    mut c: ConseilClasse,
    etablissement_id: Uuid,
) -> Result<ConseilClasse, DomainError> {
    if c.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if c.statut != ConseilClasseStatut::Planifie {
        return Err(DomainError::ConseilClasseInvalide);
    }
    c.statut = ConseilClasseStatut::Tenu;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_mention_felicitations() {
        let etab = Uuid::new_v4();
        let d = plan_ajouter_dossier_conseil(AjouterDossierConseil {
            etablissement_id: etab,
            conseil_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let decided = plan_decider_dossier_conseil(
            d.clone(),
            DeciderDossierConseil {
                etablissement_id: etab,
                dossier_id: d.id,
                appreciation: "Excellent trimestre".into(),
                mention: "felicitations".into(),
                decision_texte: String::new(),
                visible_famille: true,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(decided.mention, Some(MentionConseil::Felicitations));
        assert!(decided.visible_famille);
    }
}
