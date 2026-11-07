//! Psychologie scolaire — entretiens + planning RDV + dossier structuré (P0).
//!
//! Note **toujours** confidentielle (jamais exposée famille / prof / CPE).
//! Si `signale_famille` : le parent voit seulement jour + type, sans note.
//! RDV = créneau planifié (distinct de l'entretien réalisé).
//! Dossier : une fiche par élève ; famille voit axes + suivi si signalé — jamais note / contacts.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntretienPsyKind {
    Individuel,
    Famille,
    Orientation,
    Crise,
}

impl EntretienPsyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Individuel => "individuel",
            Self::Famille => "famille",
            Self::Orientation => "orientation",
            Self::Crise => "crise",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "individuel" => Some(Self::Individuel),
            "famille" => Some(Self::Famille),
            "orientation" => Some(Self::Orientation),
            "crise" => Some(Self::Crise),
            _ => None,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Individuel => "Individuel",
            Self::Famille => "Famille",
            Self::Orientation => "Orientation",
            Self::Crise => "Crise",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntretienPsy {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub kind: EntretienPsyKind,
    pub duree_minutes: i32,
    /// Jamais exposée hors psy / direction / admin.
    pub note_confidentielle: String,
    pub signale_famille: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerEntretienPsy {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub kind: EntretienPsyKind,
    pub duree_minutes: i32,
    pub note_confidentielle: String,
    pub signale_famille: bool,
    pub actor_user_id: Uuid,
}

/// Vue famille : type + jour + durée, note vide.
pub fn vue_famille(e: &EntretienPsy) -> EntretienPsy {
    EntretienPsy {
        note_confidentielle: String::new(),
        ..e.clone()
    }
}

pub fn plan_creer_entretien_psy(cmd: CreerEntretienPsy) -> Result<EntretienPsy, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.duree_minutes < 5 || cmd.duree_minutes > 240 {
        return Err(DomainError::EntretienPsyInvalide);
    }
    let note = cmd.note_confidentielle.trim().to_string();
    if note.is_empty() || note.len() > 8000 {
        return Err(DomainError::EntretienPsyInvalide);
    }
    Ok(EntretienPsy {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        kind: cmd.kind,
        duree_minutes: cmd.duree_minutes,
        note_confidentielle: note,
        signale_famille: cmd.signale_famille,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RdvPsyStatut {
    Planifie,
    Honore,
    Annule,
    Manque,
}

impl RdvPsyStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planifie => "planifie",
            Self::Honore => "honore",
            Self::Annule => "annule",
            Self::Manque => "manque",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "planifie" => Some(Self::Planifie),
            "honore" => Some(Self::Honore),
            "annule" => Some(Self::Annule),
            "manque" => Some(Self::Manque),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RdvPsy {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub kind: EntretienPsyKind,
    pub motif: String,
    /// Note interne psy — jamais famille.
    pub note_interne: String,
    pub statut: RdvPsyStatut,
    pub signale_famille: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerRdvPsy {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub kind: EntretienPsyKind,
    pub motif: String,
    pub note_interne: String,
    pub signale_famille: bool,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ChangerStatutRdvPsy {
    pub etablissement_id: Uuid,
    pub rdv_id: Uuid,
    pub statut: RdvPsyStatut,
    pub actor_user_id: Uuid,
}

/// Vue famille : motif + horaires, note interne vide.
pub fn vue_rdv_famille(r: &RdvPsy) -> RdvPsy {
    RdvPsy {
        note_interne: String::new(),
        ..r.clone()
    }
}

pub fn plan_creer_rdv_psy(cmd: CreerRdvPsy) -> Result<RdvPsy, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.heure_fin <= cmd.heure_debut {
        return Err(DomainError::RdvPsyInvalide);
    }
    let duree = (cmd.heure_fin - cmd.heure_debut).num_minutes();
    if !(5..=240).contains(&duree) {
        return Err(DomainError::RdvPsyInvalide);
    }
    let motif = cmd.motif.trim().to_string();
    if motif.len() > 500 {
        return Err(DomainError::RdvPsyInvalide);
    }
    let note = cmd.note_interne.trim().to_string();
    if note.len() > 4000 {
        return Err(DomainError::RdvPsyInvalide);
    }
    Ok(RdvPsy {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        heure_debut: cmd.heure_debut,
        heure_fin: cmd.heure_fin,
        kind: cmd.kind,
        motif,
        note_interne: note,
        statut: RdvPsyStatut::Planifie,
        signale_famille: cmd.signale_famille,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_changer_statut_rdv_psy(
    current: &RdvPsy,
    cmd: &ChangerStatutRdvPsy,
) -> Result<RdvPsy, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.rdv_id {
        return Err(DomainError::NotFound);
    }
    if current.statut != RdvPsyStatut::Planifie {
        return Err(DomainError::RdvPsyInvalide);
    }
    if cmd.statut == RdvPsyStatut::Planifie {
        return Err(DomainError::RdvPsyInvalide);
    }
    Ok(RdvPsy {
        statut: cmd.statut,
        ..current.clone()
    })
}

/// Dossier psy structuré — une fiche par élève (upsert).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DossierPsy {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Axes de suivi (résumé partageable si `signale_famille`).
    pub axes_suivi: String,
    /// Partenaires / contacts externes — jamais exposés famille.
    pub contacts_externes: String,
    /// Note clinique — jamais exposée famille.
    pub note_confidentielle: String,
    pub suivi_actif: bool,
    pub signale_famille: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertDossierPsy {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub axes_suivi: String,
    pub contacts_externes: String,
    pub note_confidentielle: String,
    pub suivi_actif: bool,
    pub signale_famille: bool,
    pub actor_user_id: Uuid,
}

/// Vue famille : note + contacts vidés ; axes + suivi conservés.
pub fn vue_dossier_famille(d: &DossierPsy) -> DossierPsy {
    DossierPsy {
        contacts_externes: String::new(),
        note_confidentielle: String::new(),
        ..d.clone()
    }
}

fn trim_dossier_field(s: String, max: usize) -> Result<String, DomainError> {
    let t = s.trim().to_string();
    if t.len() > max {
        return Err(DomainError::DossierPsyInvalide);
    }
    Ok(t)
}

pub fn plan_upsert_dossier_psy(cmd: UpsertDossierPsy) -> Result<DossierPsy, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let axes = trim_dossier_field(cmd.axes_suivi, 4000)?;
    let contacts = trim_dossier_field(cmd.contacts_externes, 2000)?;
    let note = trim_dossier_field(cmd.note_confidentielle, 8000)?;
    if axes.is_empty() && contacts.is_empty() && note.is_empty() && !cmd.suivi_actif {
        return Err(DomainError::DossierPsyInvalide);
    }
    Ok(DossierPsy {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        axes_suivi: axes,
        contacts_externes: contacts,
        note_confidentielle: note,
        suivi_actif: cmd.suivi_actif,
        signale_famille: cmd.signale_famille,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_duree_et_note() {
        let err = plan_creer_entretien_psy(CreerEntretienPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            kind: EntretienPsyKind::Individuel,
            duree_minutes: 2,
            note_confidentielle: "ok".into(),
            signale_famille: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EntretienPsyInvalide);
    }

    #[test]
    fn vue_famille_masque_note() {
        let e = plan_creer_entretien_psy(CreerEntretienPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            kind: EntretienPsyKind::Orientation,
            duree_minutes: 45,
            note_confidentielle: "projet 3e pro".into(),
            signale_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let v = vue_famille(&e);
        assert!(v.note_confidentielle.is_empty());
        assert_eq!(v.kind, EntretienPsyKind::Orientation);
        assert_eq!(v.duree_minutes, 45);
    }

    #[test]
    fn rdv_refuse_horaires_inverses() {
        let err = plan_creer_rdv_psy(CreerRdvPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            heure_debut: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            heure_fin: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            kind: EntretienPsyKind::Individuel,
            motif: "suivi".into(),
            note_interne: String::new(),
            signale_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::RdvPsyInvalide);
    }

    #[test]
    fn rdv_honorer_ok() {
        let r = plan_creer_rdv_psy(CreerRdvPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            heure_fin: NaiveTime::from_hms_opt(14, 45, 0).unwrap(),
            kind: EntretienPsyKind::Famille,
            motif: "point orientation".into(),
            note_interne: "préparer dossier".into(),
            signale_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let h = plan_changer_statut_rdv_psy(
            &r,
            &ChangerStatutRdvPsy {
                etablissement_id: r.etablissement_id,
                rdv_id: r.id,
                statut: RdvPsyStatut::Honore,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(h.statut, RdvPsyStatut::Honore);
        let v = vue_rdv_famille(&r);
        assert!(v.note_interne.is_empty());
        assert_eq!(v.motif, "point orientation");
    }

    #[test]
    fn refuse_dossier_vide() {
        let err = plan_upsert_dossier_psy(UpsertDossierPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            axes_suivi: "  ".into(),
            contacts_externes: String::new(),
            note_confidentielle: String::new(),
            suivi_actif: false,
            signale_famille: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::DossierPsyInvalide);
    }

    #[test]
    fn vue_dossier_masque_note_et_contacts() {
        let d = plan_upsert_dossier_psy(UpsertDossierPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            axes_suivi: "anxiété scolaire · orientation".into(),
            contacts_externes: "CMP Dr X".into(),
            note_confidentielle: "hypothèse clinique".into(),
            suivi_actif: true,
            signale_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let v = vue_dossier_famille(&d);
        assert!(v.note_confidentielle.is_empty());
        assert!(v.contacts_externes.is_empty());
        assert_eq!(v.axes_suivi, "anxiété scolaire · orientation");
        assert!(v.suivi_actif);
    }
}
