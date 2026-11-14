//! Documents à signer (famille) — P1.
//!
//! Staff publie un document (règlement, charte…) vers un élève ;
//! parent signe ; file VS des non-signés.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocSignerStatut {
    ASigner,
    Signe,
    Annule,
}

impl DocSignerStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ASigner => "a_signer",
            Self::Signe => "signe",
            Self::Annule => "annule",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "a_signer" | "pending" => Some(Self::ASigner),
            "signe" | "signé" | "signed" => Some(Self::Signe),
            "annule" | "annulé" | "cancelled" => Some(Self::Annule),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocASigner {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub titre: String,
    pub corps: String,
    pub annee_scolaire: String,
    pub statut: DocSignerStatut,
    pub publie_par_user_id: Uuid,
    pub publie_at: DateTime<Utc>,
    pub echeance: Option<NaiveDate>,
    pub signe_at: Option<DateTime<Utc>>,
    pub signe_par_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct PublierDocASigner {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub titre: String,
    pub corps: String,
    pub annee_scolaire: String,
    pub echeance: Option<NaiveDate>,
    pub actor_user_id: Uuid,
}

pub fn plan_publier_doc_a_signer(cmd: PublierDocASigner) -> Result<DocASigner, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    let titre = cmd.titre.trim().to_string();
    let corps = cmd.corps.trim().to_string();
    let annee = cmd.annee_scolaire.trim().to_string();
    if code.is_empty() || titre.is_empty() || corps.is_empty() || annee.is_empty() {
        return Err(DomainError::DocASignerInvalide);
    }
    if code.len() > 64 || titre.len() > 200 || corps.len() > 20_000 {
        return Err(DomainError::DocASignerInvalide);
    }
    Ok(DocASigner {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        code,
        titre,
        corps,
        annee_scolaire: annee,
        statut: DocSignerStatut::ASigner,
        publie_par_user_id: cmd.actor_user_id,
        publie_at: Utc::now(),
        echeance: cmd.echeance,
        signe_at: None,
        signe_par_user_id: None,
    })
}

#[derive(Debug, Clone)]
pub struct SignerDocASigner {
    pub etablissement_id: Uuid,
    pub doc_id: Uuid,
    pub actor_user_id: Uuid,
}

pub fn plan_signer_doc_a_signer(
    current: DocASigner,
    cmd: SignerDocASigner,
) -> Result<DocASigner, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.doc_id {
        return Err(DomainError::NotFound);
    }
    match current.statut {
        DocSignerStatut::Signe => return Err(DomainError::DocASignerDejaSigne),
        DocSignerStatut::Annule => return Err(DomainError::DocASignerInvalide),
        DocSignerStatut::ASigner => {}
    }
    Ok(DocASigner {
        statut: DocSignerStatut::Signe,
        signe_at: Some(Utc::now()),
        signe_par_user_id: Some(cmd.actor_user_id),
        ..current
    })
}

pub fn plan_annuler_doc_a_signer(
    mut current: DocASigner,
    etablissement_id: Uuid,
) -> Result<DocASigner, DomainError> {
    if current.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.statut != DocSignerStatut::ASigner {
        return Err(DomainError::DocASignerInvalide);
    }
    current.statut = DocSignerStatut::Annule;
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publier_signer_idempotent_refuse() {
        let etab = Uuid::new_v4();
        let d = plan_publier_doc_a_signer(PublierDocASigner {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            code: "reglement".into(),
            titre: "Règlement intérieur".into(),
            corps: "Article 1…".into(),
            annee_scolaire: "2026-2027".into(),
            echeance: None,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let signed = plan_signer_doc_a_signer(
            d.clone(),
            SignerDocASigner {
                etablissement_id: etab,
                doc_id: d.id,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(signed.statut, DocSignerStatut::Signe);
        let err = plan_signer_doc_a_signer(
            signed,
            SignerDocASigner {
                etablissement_id: etab,
                doc_id: d.id,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::DocASignerDejaSigne);
    }
}
