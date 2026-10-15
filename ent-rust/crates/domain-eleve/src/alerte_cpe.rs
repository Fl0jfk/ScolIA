//! Alertes CPE / Vie scolaire (P2) — file d'attention hors seuils absences.
//!
//! CPE pose une alerte (comportement, retard chronique, suivi…) ; clôture manuelle.
//! Famille voit uniquement si `visible_famille`.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlerteCpeSeverite {
    Info,
    Attention,
    Urgent,
}

impl AlerteCpeSeverite {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Attention => "attention",
            Self::Urgent => "urgent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "info" | "basse" => Some(Self::Info),
            "attention" | "normale" | "medium" => Some(Self::Attention),
            "urgent" | "haute" | "critique" => Some(Self::Urgent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlerteCpeStatut {
    Ouverte,
    Close,
}

impl AlerteCpeStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ouverte => "ouverte",
            Self::Close => "close",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ouverte" | "open" => Some(Self::Ouverte),
            "close" | "closee" | "clôturée" | "cloturee" => Some(Self::Close),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlerteCpe {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub detail: String,
    pub severite: AlerteCpeSeverite,
    pub statut: AlerteCpeStatut,
    pub visible_famille: bool,
    pub jour: NaiveDate,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_by_user_id: Option<Uuid>,
    pub closed_at: Option<DateTime<Utc>>,
    pub cloture_note: String,
}

#[derive(Debug, Clone)]
pub struct CreerAlerteCpe {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub detail: String,
    pub severite: String,
    pub visible_famille: bool,
    pub jour: NaiveDate,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_alerte_cpe(cmd: CreerAlerteCpe) -> Result<AlerteCpe, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let titre = cmd.titre.trim().to_string();
    let detail = cmd.detail.trim().to_string();
    if titre.is_empty() || titre.len() > 200 {
        return Err(DomainError::AlerteCpeInvalide);
    }
    if detail.len() > 2000 {
        return Err(DomainError::AlerteCpeInvalide);
    }
    let severite = AlerteCpeSeverite::parse(&cmd.severite).ok_or(DomainError::AlerteCpeInvalide)?;
    Ok(AlerteCpe {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        titre,
        detail,
        severite,
        statut: AlerteCpeStatut::Ouverte,
        visible_famille: cmd.visible_famille,
        jour: cmd.jour,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
        closed_by_user_id: None,
        closed_at: None,
        cloture_note: String::new(),
    })
}

pub fn plan_clore_alerte_cpe(
    mut current: AlerteCpe,
    etablissement_id: Uuid,
    actor_user_id: Uuid,
    note: String,
) -> Result<AlerteCpe, DomainError> {
    if current.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.statut == AlerteCpeStatut::Close {
        return Err(DomainError::AlerteCpeDejaClose);
    }
    let note = note.trim().to_string();
    if note.len() > 500 {
        return Err(DomainError::AlerteCpeInvalide);
    }
    current.statut = AlerteCpeStatut::Close;
    current.closed_by_user_id = Some(actor_user_id);
    current.closed_at = Some(Utc::now());
    current.cloture_note = note;
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creer_et_clore() {
        let etab = Uuid::new_v4();
        let a = plan_creer_alerte_cpe(CreerAlerteCpe {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            titre: "Retards répétés".into(),
            detail: "3e retard de la semaine".into(),
            severite: "attention".into(),
            visible_famille: true,
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(a.statut, AlerteCpeStatut::Ouverte);
        let closed = plan_clore_alerte_cpe(a, etab, Uuid::new_v4(), "Entretien fait".into()).unwrap();
        assert_eq!(closed.statut, AlerteCpeStatut::Close);
    }
}
