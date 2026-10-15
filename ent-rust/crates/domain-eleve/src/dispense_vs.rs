//! Dispenses Vie scolaire (EPS / piscine / autre) — P1.
//!
//! Parent ou staff pose une dispense sur une période ; CPE/staff peut clore.
//! L'appel / EDT consomment `dispense_vs_couvre` pour marquer l'élève dispensé.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispenseVsMotif {
    Eps,
    Piscine,
    Sport,
    Medical,
    Autre,
}

impl DispenseVsMotif {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eps => "eps",
            Self::Piscine => "piscine",
            Self::Sport => "sport",
            Self::Medical => "medical",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "eps" | "eps_sport" => Some(Self::Eps),
            "piscine" | "natation" => Some(Self::Piscine),
            "sport" => Some(Self::Sport),
            "medical" | "médical" | "sante" | "santé" => Some(Self::Medical),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }

    /// Matières EDT typiquement couvertes par ce motif.
    pub fn matieres_couvertes(self) -> &'static [&'static str] {
        match self {
            Self::Eps | Self::Sport => &["EPS", "Sport", "Éducation physique"],
            Self::Piscine => &["EPS", "Piscine", "Natation"],
            Self::Medical | Self::Autre => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispenseVsStatut {
    Active,
    Close,
}

impl DispenseVsStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Close => "close",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "close" | "cloturee" | "clôturée" => Some(Self::Close),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispenseVs {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub motif: DispenseVsMotif,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub commentaire: String,
    pub statut: DispenseVsStatut,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_by_user_id: Option<Uuid>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl DispenseVs {
    pub fn couvre_jour(&self, jour: NaiveDate) -> bool {
        self.statut == DispenseVsStatut::Active && self.debut <= jour && jour <= self.fin
    }

    pub fn couvre_matiere(&self, matiere_libelle: &str) -> bool {
        let mats = self.motif.matieres_couvertes();
        if mats.is_empty() {
            return true; // médical / autre = toutes activités physiques par défaut
        }
        let needle = matiere_libelle.trim().to_ascii_lowercase();
        mats.iter()
            .any(|m| needle.contains(&m.to_ascii_lowercase()) || m.to_ascii_lowercase().contains(&needle))
    }
}

#[derive(Debug, Clone)]
pub struct PoserDispenseVs {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub motif: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub commentaire: String,
    pub actor_user_id: Uuid,
}

pub fn plan_poser_dispense_vs(cmd: PoserDispenseVs) -> Result<DispenseVs, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::DispenseVsInvalide);
    }
    let motif = DispenseVsMotif::parse(&cmd.motif).ok_or(DomainError::DispenseVsInvalide)?;
    let commentaire = cmd.commentaire.trim().to_string();
    if commentaire.len() > 1000 {
        return Err(DomainError::DispenseVsInvalide);
    }
    Ok(DispenseVs {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        motif,
        debut: cmd.debut,
        fin: cmd.fin,
        commentaire,
        statut: DispenseVsStatut::Active,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
        closed_by_user_id: None,
        closed_at: None,
    })
}

pub fn plan_clore_dispense_vs(
    mut current: DispenseVs,
    etablissement_id: Uuid,
    actor_user_id: Uuid,
) -> Result<DispenseVs, DomainError> {
    if current.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.statut == DispenseVsStatut::Close {
        return Err(DomainError::DispenseVsDejaClose);
    }
    current.statut = DispenseVsStatut::Close;
    current.closed_by_user_id = Some(actor_user_id);
    current.closed_at = Some(Utc::now());
    Ok(current)
}

pub fn dispense_vs_couvre<'a>(
    dispenses: &'a [DispenseVs],
    jour: NaiveDate,
    matiere_libelle: Option<&str>,
) -> Option<&'a DispenseVs> {
    dispenses.iter().find(|d| {
        d.couvre_jour(jour)
            && matiere_libelle
                .map(|m| d.couvre_matiere(m))
                .unwrap_or(true)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eps_couvre_matiere_et_periode() {
        let d = plan_poser_dispense_vs(PoserDispenseVs {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "eps".into(),
            debut: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            commentaire: "entorse".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert!(d.couvre_jour(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()));
        assert!(!d.couvre_jour(NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()));
        assert!(d.couvre_matiere("EPS 4B"));
        assert!(!d.couvre_matiere("Mathématiques"));
    }

    #[test]
    fn medical_couvre_toute_matiere() {
        let d = plan_poser_dispense_vs(PoserDispenseVs {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "medical".into(),
            debut: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
            commentaire: String::new(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert!(d.couvre_matiere("Histoire"));
        let closed = plan_clore_dispense_vs(d.clone(), d.etablissement_id, Uuid::new_v4()).unwrap();
        assert_eq!(closed.statut, DispenseVsStatut::Close);
        assert!(!closed.couvre_jour(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()));
    }
}
