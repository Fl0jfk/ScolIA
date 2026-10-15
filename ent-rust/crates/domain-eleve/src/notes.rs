//! Notes — périodes (verrou) + devoirs + moyenne pondérée.
//!
//! Bulletins / édition = plus tard. Ici : saisie rattachée à un devoir
//! dans une période ouverte, et lecture des moyennes matière.

use crate::DomainError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotePeriodeStatut {
    Ouverte,
    Fermee,
    PartielleAppreciations,
}

impl NotePeriodeStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ouverte => "ouverte",
            Self::Fermee => "fermee",
            Self::PartielleAppreciations => "partielle_appreciations",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ouverte" => Some(Self::Ouverte),
            "fermee" => Some(Self::Fermee),
            "partielle_appreciations" => Some(Self::PartielleAppreciations),
            _ => None,
        }
    }

    /// Saisie de notes autorisée ?
    pub fn autorise_saisie_notes(self) -> bool {
        matches!(self, Self::Ouverte)
    }

    /// Saisie d'appréciations autorisée (ouverte ou phase partielle).
    pub fn autorise_appreciations(self) -> bool {
        matches!(self, Self::Ouverte | Self::PartielleAppreciations)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotePeriode {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub statut: NotePeriodeStatut,
}

#[derive(Debug, Clone)]
pub struct CreerNotePeriode {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Devoir {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    pub classe: String,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
    pub created_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct CreerDevoir {
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    pub classe: String,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
    pub actor_user_id: Uuid,
}

/// Note telle que lue sur le dossier (sans auteur).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteLue {
    pub id: Uuid,
    pub matiere_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matiere_libelle: Option<String>,
    pub valeur: f64,
    pub sur: f64,
    pub date_note: NaiveDate,
    pub libelle: Option<String>,
    pub devoir_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
}

/// Moyenne matière sur une période, ramenée sur 20.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoyenneMatiere {
    pub matiere_id: Uuid,
    pub periode_id: Uuid,
    pub moyenne_sur_20: f64,
}

/// Ligne pour calcul de moyenne : (valeur, barème, coefficient).
pub type NotePourMoyenne = (f64, f64, f64);

/// Moyenne ramenée sur 20, pondérée par les coefs. `None` si aucune note.
pub fn moyenne_sur_20(notes: &[NotePourMoyenne]) -> Option<f64> {
    let mut sum_w = 0.0;
    let mut sum_c = 0.0;
    for &(valeur, sur, coef) in notes {
        if sur <= 0.0 || coef <= 0.0 {
            continue;
        }
        sum_w += (valeur / sur) * 20.0 * coef;
        sum_c += coef;
    }
    if sum_c <= 0.0 {
        None
    } else {
        Some(sum_w / sum_c)
    }
}

pub fn plan_creer_note_periode(cmd: CreerNotePeriode) -> Result<NotePeriode, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || libelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    if cmd.date_fin < cmd.date_debut {
        return Err(DomainError::NotePeriodeDatesInvalides);
    }
    Ok(NotePeriode {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        date_debut: cmd.date_debut,
        date_fin: cmd.date_fin,
        statut: NotePeriodeStatut::Ouverte,
    })
}

pub fn plan_creer_devoir(cmd: CreerDevoir) -> Result<Devoir, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let titre = cmd.titre.trim().to_string();
    let classe = cmd.classe.trim().to_string();
    if titre.is_empty() || classe.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    if cmd.coef <= 0.0 || cmd.sur <= 0.0 {
        return Err(DomainError::NoteBaremeInvalide);
    }
    Ok(Devoir {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        periode_id: cmd.periode_id,
        matiere_id: cmd.matiere_id,
        classe,
        titre,
        date_devoir: cmd.date_devoir,
        coef: cmd.coef,
        sur: cmd.sur,
        created_by_user_id: Some(cmd.actor_user_id),
    })
}

/// Appréciation bulletin — matière (`matiere_id`) ou générale (`None`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Appreciation {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub texte: String,
    pub auteur_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct SaisirAppreciation {
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub texte: String,
    pub actor_user_id: Uuid,
}

const APPRECIATION_MAX: usize = 4000;

pub fn plan_saisir_appreciation(
    periode: &NotePeriode,
    cmd: SaisirAppreciation,
) -> Result<Appreciation, DomainError> {
    if periode.etablissement_id != cmd.etablissement_id || periode.id != cmd.periode_id {
        return Err(DomainError::TenantMismatch);
    }
    if !periode.statut.autorise_appreciations() {
        return Err(DomainError::NoteAppreciationPeriodeFermee);
    }
    let texte = cmd.texte.trim().to_string();
    if texte.is_empty() {
        return Err(DomainError::NoteAppreciationVide);
    }
    if texte.chars().count() > APPRECIATION_MAX {
        return Err(DomainError::NoteAppreciationTropLongue);
    }
    Ok(Appreciation {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        periode_id: cmd.periode_id,
        eleve_id: cmd.eleve_id,
        matiere_id: cmd.matiere_id,
        texte,
        auteur_user_id: cmd.actor_user_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn moyenne_ponderee() {
        // 10/20 coef 1 + 20/20 coef 1 → 15
        let m = moyenne_sur_20(&[(10.0, 20.0, 1.0), (20.0, 20.0, 1.0)]).unwrap();
        assert!((m - 15.0).abs() < 1e-9);
        // 8/10 coef 2 (=16/20) + 10/20 coef 1 → (32+10)/3 = 14
        let m2 = moyenne_sur_20(&[(8.0, 10.0, 2.0), (10.0, 20.0, 1.0)]).unwrap();
        assert!((m2 - 14.0).abs() < 1e-9);
    }

    #[test]
    fn periode_fermee_bloque_saisie() {
        assert!(!NotePeriodeStatut::Fermee.autorise_saisie_notes());
        assert!(!NotePeriodeStatut::PartielleAppreciations.autorise_saisie_notes());
        assert!(NotePeriodeStatut::Ouverte.autorise_saisie_notes());
        assert!(NotePeriodeStatut::PartielleAppreciations.autorise_appreciations());
        assert!(!NotePeriodeStatut::Fermee.autorise_appreciations());
    }

    #[test]
    fn appreciation_refuse_si_fermee() {
        let periode = NotePeriode {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            code: "T1".into(),
            libelle: "Trimestre 1".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: NaiveDate::from_ymd_opt(2026, 12, 18).unwrap(),
            statut: NotePeriodeStatut::Fermee,
        };
        let err = plan_saisir_appreciation(
            &periode,
            SaisirAppreciation {
                etablissement_id: periode.etablissement_id,
                periode_id: periode.id,
                eleve_id: Uuid::new_v4(),
                matiere_id: None,
                texte: "Bon trimestre.".into(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::NoteAppreciationPeriodeFermee);
    }
}
