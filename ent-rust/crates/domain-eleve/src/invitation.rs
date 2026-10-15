//! Invitation / provisionnement compte face élève.
//!
//! P0 : création immédiate du compte (mdp temporaire renvoyé une fois).
//! Pas d'envoi SMTP ici — traçabilité + geste staff observable.

use crate::{DomainError, Eleve, EleveStatus};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatut {
    /// Compte créé et lié immédiatement (mdp temporaire remis au staff).
    Provisionne,
    Revoquee,
}

impl InvitationStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provisionne => "provisionne",
            Self::Revoquee => "revoquee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "provisionne" => Some(Self::Provisionne),
            "revoquee" => Some(Self::Revoquee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveInvitation {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub email: String,
    pub statut: InvitationStatut,
    pub invited_by_user_id: Uuid,
    pub activated_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InviterCompteEleve {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Si absent : email dérivé prenom.nom + suffixe établissement.
    pub email: Option<String>,
    pub actor_user_id: Uuid,
}

/// Résultat domaine avant persistance auth : invitation + email + mdp temporaire.
#[derive(Debug, Clone)]
pub struct InvitationPlanifiee {
    pub invitation: EleveInvitation,
    pub email: String,
    pub mot_de_passe_temporaire: String,
    pub display_name: String,
}

fn slug_part(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect::<String>()
}

/// Email local dérivé (pas de SMTP) — unique via suffixe eleve_id.
pub fn email_compte_eleve_defaut(eleve: &Eleve) -> String {
    let p = slug_part(&eleve.prenom);
    let n = slug_part(&eleve.nom);
    let short = &eleve.id.to_string()[..8];
    let local = if p.is_empty() && n.is_empty() {
        format!("eleve-{short}")
    } else if p.is_empty() {
        format!("{n}-{short}")
    } else if n.is_empty() {
        format!("{p}-{short}")
    } else {
        format!("{p}.{n}-{short}")
    };
    format!("{local}@ent.lab")
}

pub fn generer_mot_de_passe_temporaire() -> String {
    let raw = Uuid::new_v4().simple().to_string();
    format!("Eleve{}!", &raw[..12])
}

pub fn plan_inviter_compte_eleve(
    eleve: &Eleve,
    deja_compte: bool,
    cmd: InviterCompteEleve,
) -> Result<InvitationPlanifiee, DomainError> {
    if eleve.etablissement_id != cmd.etablissement_id || eleve.id != cmd.eleve_id {
        return Err(DomainError::TenantMismatch);
    }
    if eleve.status != EleveStatus::Actif {
        return Err(DomainError::InvitationEleveInactif);
    }
    if deja_compte {
        return Err(DomainError::InvitationCompteDejaLie);
    }
    let email = match cmd.email {
        Some(ref e) => {
            let t = e.trim().to_lowercase();
            if t.is_empty() || !t.contains('@') || t.len() > 200 {
                return Err(DomainError::InvitationEmailInvalide);
            }
            t
        }
        None => email_compte_eleve_defaut(eleve),
    };
    let now = Utc::now();
    let invitation = EleveInvitation {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        email: email.clone(),
        statut: InvitationStatut::Provisionne,
        invited_by_user_id: cmd.actor_user_id,
        activated_user_id: None, // rempli après register
        created_at: now,
        expires_at: now + Duration::days(30),
    };
    Ok(InvitationPlanifiee {
        invitation,
        email,
        mot_de_passe_temporaire: generer_mot_de_passe_temporaire(),
        display_name: format!("{} {}", eleve.prenom.trim(), eleve.nom.trim())
            .trim()
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EleveStatus;

    fn eleve_actif(etab: Uuid, id: Uuid) -> Eleve {
        Eleve {
            id,
            etablissement_id: etab,
            nom: "Potter".into(),
            prenom: "Harry".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            photo_key: None,
            status: EleveStatus::Actif,
            classe: Some("4B".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
        }
    }

    #[test]
    fn refuse_deja_lie_et_email_invalide() {
        let etab = Uuid::new_v4();
        let id = Uuid::new_v4();
        let e = eleve_actif(etab, id);
        let err = plan_inviter_compte_eleve(
            &e,
            true,
            InviterCompteEleve {
                etablissement_id: etab,
                eleve_id: id,
                email: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::InvitationCompteDejaLie);

        let err2 = plan_inviter_compte_eleve(
            &e,
            false,
            InviterCompteEleve {
                etablissement_id: etab,
                eleve_id: id,
                email: Some("pas-un-mail".into()),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err2, DomainError::InvitationEmailInvalide);
    }

    #[test]
    fn provision_ok_email_defaut() {
        let etab = Uuid::new_v4();
        let id = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        let e = eleve_actif(etab, id);
        let plan = plan_inviter_compte_eleve(
            &e,
            false,
            InviterCompteEleve {
                etablissement_id: etab,
                eleve_id: id,
                email: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert!(plan.email.starts_with("harry.potter-aaaaaaaa@"));
        assert!(plan.mot_de_passe_temporaire.len() >= 10);
        assert_eq!(plan.invitation.statut, InvitationStatut::Provisionne);
    }
}
