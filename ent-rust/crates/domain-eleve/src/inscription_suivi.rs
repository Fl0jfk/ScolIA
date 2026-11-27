//! Workflow suivi inscription — étapes + motifs (P0).

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InscriptionEtapeKind {
    Dossier,
    Pieces,
    Entretien,
    Decision,
    Paiement,
    Finalise,
}

impl InscriptionEtapeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dossier => "dossier",
            Self::Pieces => "pieces",
            Self::Entretien => "entretien",
            Self::Decision => "decision",
            Self::Paiement => "paiement",
            Self::Finalise => "finalise",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dossier" => Some(Self::Dossier),
            "pieces" | "pièces" => Some(Self::Pieces),
            "entretien" | "rdv" => Some(Self::Entretien),
            "decision" | "décision" => Some(Self::Decision),
            "paiement" => Some(Self::Paiement),
            "finalise" | "finalisé" | "termine" => Some(Self::Finalise),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InscriptionSuivi {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub etape: InscriptionEtapeKind,
    pub motif_code: Option<String>,
    pub commentaire: String,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertInscriptionSuivi {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub etape: String,
    pub motif_code: Option<String>,
    pub commentaire: String,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_inscription_suivi(
    cmd: UpsertInscriptionSuivi,
) -> Result<InscriptionSuivi, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let annee = cmd.annee_scolaire.trim().to_string();
    if annee.is_empty() || annee.len() > 20 {
        return Err(DomainError::InscriptionSuiviInvalide);
    }
    let etape = InscriptionEtapeKind::parse(&cmd.etape).ok_or(DomainError::InscriptionSuiviInvalide)?;
    let motif = cmd
        .motif_code
        .map(|m| m.trim().to_ascii_lowercase())
        .filter(|m| !m.is_empty());
    if motif.as_ref().is_some_and(|m| m.len() > 64) {
        return Err(DomainError::InscriptionSuiviInvalide);
    }
    let commentaire = cmd.commentaire.trim().to_string();
    if commentaire.len() > 2000 {
        return Err(DomainError::InscriptionSuiviInvalide);
    }
    Ok(InscriptionSuivi {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        annee_scolaire: annee,
        etape,
        motif_code: motif,
        commentaire,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etape_pieces() {
        let s = plan_upsert_inscription_suivi(UpsertInscriptionSuivi {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            annee_scolaire: "2026-2027".into(),
            etape: "pièces".into(),
            motif_code: Some("incomplet".into()),
            commentaire: "Attente livret".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(s.etape, InscriptionEtapeKind::Pieces);
    }
}
