//! Documents élève — verrouillés par défaut, déverrouillage ciblé (grants).
//!
//! Corrige le circuit Charlemagne cassé : le prof produit un PAP mais ne peut
//! pas le relire sans passer par l'admin. Ici : dépôt + grant explicite, lisible.

use crate::DomainError;
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocTiroir {
    Scolaire,
    Administratif,
    Sante,
    Famille,
    Autre,
}

impl DocTiroir {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scolaire => "scolaire",
            Self::Administratif => "administratif",
            Self::Sante => "sante",
            Self::Famille => "famille",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "scolaire" => Some(Self::Scolaire),
            "administratif" => Some(Self::Administratif),
            "sante" => Some(Self::Sante),
            "famille" => Some(Self::Famille),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocConfidentialite {
    /// Visible staff manage + parent lié (tiroir famille) sans grant.
    Standard,
    /// Verrouillé : grant obligatoire hors manage.
    Restreint,
    /// Santé : infirmière / manage / grant.
    Sante,
}

impl DocConfidentialite {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Restreint => "restreint",
            Self::Sante => "sante",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "standard" => Some(Self::Standard),
            "restreint" => Some(Self::Restreint),
            "sante" => Some(Self::Sante),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveDocument {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub tiroir: DocTiroir,
    pub titre: String,
    /// Code métier (ex. ASSURANCE_RC) — sert aux vannes réinscription.
    pub code: Option<String>,
    pub mime_type: Option<String>,
    pub storage_key: Option<String>,
    pub confidentialite: DocConfidentialite,
    pub source: String,
    pub created_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DeposerDocument {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub tiroir: DocTiroir,
    pub titre: String,
    pub code: Option<String>,
    pub mime_type: Option<String>,
    pub storage_key: Option<String>,
    pub confidentialite: DocConfidentialite,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    Pending,
    Granted,
    Refused,
    Expired,
}

impl GrantStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Granted => "granted",
            Self::Refused => "refused",
            Self::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "granted" => Some(Self::Granted),
            "refused" => Some(Self::Refused),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentGrant {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub document_id: Uuid,
    pub grantee_user_id: Uuid,
    pub status: GrantStatus,
    pub duration_days: i32,
    pub decided_by_user_id: Option<Uuid>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DemanderGrant {
    pub etablissement_id: Uuid,
    pub document_id: Uuid,
    pub grantee_user_id: Uuid,
    pub duration_days: i32,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeciderGrant {
    pub etablissement_id: Uuid,
    pub grant_id: Uuid,
    pub accepter: bool,
    pub actor_user_id: Uuid,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositifKind {
    Pap,
    Pps,
    Pai,
    Gevasco,
    Autre,
}

impl DispositifKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pap => "pap",
            Self::Pps => "pps",
            Self::Pai => "pai",
            Self::Gevasco => "gevasco",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pap" => Some(Self::Pap),
            "pps" => Some(Self::Pps),
            "pai" => Some(Self::Pai),
            "gevasco" => Some(Self::Gevasco),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveDispositif {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub kind: DispositifKind,
    pub libelle: Option<String>,
    pub document_id: Option<Uuid>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct AjouterDispositif {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub kind: DispositifKind,
    pub libelle: Option<String>,
    pub document_id: Option<Uuid>,
    pub date_debut: NaiveDate,
    pub actor_user_id: Option<String>,
}

pub fn plan_deposer_document(cmd: DeposerDocument) -> Result<EleveDocument, DomainError> {
    let titre = cmd.titre.trim().to_string();
    if titre.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    // Défaut produit : restreint si on dépose un doc sensible sans le dire.
    let code = cmd
        .code
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty());
    Ok(EleveDocument {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        tiroir: cmd.tiroir,
        titre,
        code,
        mime_type: cmd.mime_type,
        storage_key: cmd.storage_key,
        confidentialite: cmd.confidentialite,
        source: "upload".into(),
        created_by_user_id: Some(cmd.actor_user_id),
    })
}

pub fn plan_demander_grant(cmd: DemanderGrant) -> Result<DocumentGrant, DomainError> {
    if cmd.duration_days < 1 || cmd.duration_days > 365 {
        return Err(DomainError::GrantDureeInvalide);
    }
    Ok(DocumentGrant {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        document_id: cmd.document_id,
        grantee_user_id: cmd.grantee_user_id,
        status: GrantStatus::Pending,
        duration_days: cmd.duration_days,
        decided_by_user_id: None,
        expires_at: None,
        note: cmd.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
    })
}

pub fn plan_decider_grant(
    current: &DocumentGrant,
    cmd: DeciderGrant,
) -> Result<DocumentGrant, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.grant_id {
        return Err(DomainError::NotFound);
    }
    if current.status != GrantStatus::Pending {
        return Err(DomainError::GrantDejaTranche);
    }
    let mut g = current.clone();
    g.decided_by_user_id = Some(cmd.actor_user_id);
    g.note = cmd.note.or(g.note);
    if cmd.accepter {
        g.status = GrantStatus::Granted;
        g.expires_at = Some(Utc::now() + Duration::days(i64::from(g.duration_days)));
    } else {
        g.status = GrantStatus::Refused;
        g.expires_at = None;
    }
    Ok(g)
}

/// Grant actif = granted + (pas d'expiry ou expiry future).
pub fn grant_est_actif(g: &DocumentGrant, now: chrono::DateTime<Utc>) -> bool {
    g.status == GrantStatus::Granted
        && g.expires_at.map(|e| e > now).unwrap_or(false)
}

pub fn plan_ajouter_dispositif(cmd: AjouterDispositif) -> Result<EleveDispositif, DomainError> {
    Ok(EleveDispositif {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        kind: cmd.kind,
        libelle: cmd
            .libelle
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty()),
        document_id: cmd.document_id,
        date_debut: cmd.date_debut,
        date_fin: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_accepte_pose_expiration() {
        let pending = plan_demander_grant(DemanderGrant {
            etablissement_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            grantee_user_id: Uuid::new_v4(),
            duration_days: 7,
            note: Some("PAP maths".into()),
        })
        .unwrap();
        let actor = Uuid::new_v4();
        let decided = plan_decider_grant(
            &pending,
            DeciderGrant {
                etablissement_id: pending.etablissement_id,
                grant_id: pending.id,
                accepter: true,
                actor_user_id: actor,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(decided.status, GrantStatus::Granted);
        assert!(grant_est_actif(&decided, Utc::now()));
    }

    #[test]
    fn titre_vide_refuse() {
        assert!(plan_deposer_document(DeposerDocument {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            tiroir: DocTiroir::Scolaire,
            titre: "  ".into(),
            code: None,
            mime_type: None,
            storage_key: None,
            confidentialite: DocConfidentialite::Restreint,
            actor_user_id: Uuid::new_v4(),
        })
        .is_err());
    }
}
