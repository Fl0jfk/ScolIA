//! Internat — chambres, affectations, appel de présence, suivis (P1).
//!
//! Geste VS : créer des chambres → affecter un interne → pointer le soir → noter un suivi.
//! Famille : lit la chambre + présence (pas les notes internes).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChambreGenre {
    Mixte,
    Filles,
    Garcons,
}

impl ChambreGenre {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mixte => "mixte",
            Self::Filles => "filles",
            Self::Garcons => "garcons",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mixte" | "m" => Some(Self::Mixte),
            "filles" | "f" | "fille" => Some(Self::Filles),
            "garcons" | "garçons" | "g" | "garcon" | "garçon" => Some(Self::Garcons),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChambreInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub genre: ChambreGenre,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerChambreInternat {
    pub etablissement_id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub genre: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_chambre_internat(
    cmd: CreerChambreInternat,
) -> Result<ChambreInternat, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let batiment = cmd.batiment.trim().to_string();
    let code = cmd.code.trim().to_string();
    if batiment.is_empty() || code.is_empty() {
        return Err(DomainError::InternatChambreInvalide);
    }
    if cmd.capacite < 1 || cmd.capacite > 64 {
        return Err(DomainError::InternatChambreInvalide);
    }
    let genre = ChambreGenre::parse(&cmd.genre).ok_or(DomainError::InternatChambreInvalide)?;
    Ok(ChambreInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        batiment,
        etage: cmd.etage,
        code,
        capacite: cmd.capacite,
        genre,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AffectationInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit: Option<String>,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl AffectationInternat {
    pub fn est_active_au(&self, jour: NaiveDate) -> bool {
        self.debut <= jour && self.fin.map(|f| jour <= f).unwrap_or(true)
    }
}

#[derive(Debug, Clone)]
pub struct AffecterEleveInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit: Option<String>,
    pub debut: NaiveDate,
    pub actor_user_id: Uuid,
}

/// Vérifie capacité + régime INT avant insertion.
pub fn plan_affecter_eleve_internat(
    cmd: AffecterEleveInternat,
    chambre: &ChambreInternat,
    regime_eleve: Option<&str>,
    occupants_actifs: i16,
    deja_affecte: bool,
) -> Result<AffectationInternat, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() || cmd.chambre_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if chambre.etablissement_id != cmd.etablissement_id || chambre.id != cmd.chambre_id {
        return Err(DomainError::InternatChambreInvalide);
    }
    if !chambre.actif {
        return Err(DomainError::InternatChambreInvalide);
    }
    if deja_affecte {
        return Err(DomainError::InternatDejaAffecte);
    }
    let regime = normalize_regime_internat(regime_eleve.unwrap_or(""));
    if regime != "INT" {
        return Err(DomainError::InternatRegimeRequis);
    }
    if occupants_actifs >= chambre.capacite {
        return Err(DomainError::InternatChambrePleine);
    }
    let lit = cmd
        .lit
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(AffectationInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        chambre_id: cmd.chambre_id,
        lit,
        debut: cmd.debut,
        fin: None,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_clore_affectation_internat(
    mut current: AffectationInternat,
    etablissement_id: Uuid,
    fin: NaiveDate,
) -> Result<AffectationInternat, DomainError> {
    if current.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.fin.is_some() {
        return Err(DomainError::InternatAffectationDejaClose);
    }
    if fin < current.debut {
        return Err(DomainError::InternatAffectationInvalide);
    }
    current.fin = Some(fin);
    Ok(current)
}

pub fn normalize_regime_internat(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "int" | "internat" | "interne" | "i" => "INT".into(),
        "dp" | "demi-pension" | "demi_pension" | "1/2p" | "1/2 p" => "DP".into(),
        "ext" | "externe" | "e" => "EXT".into(),
        other if !other.is_empty() => other.to_ascii_uppercase(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceInternatStatut {
    Present,
    Absent,
    PermisSortie,
    Retard,
}

impl PresenceInternatStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::PermisSortie => "permis_sortie",
            Self::Retard => "retard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "present" | "présent" => Some(Self::Present),
            "absent" => Some(Self::Absent),
            "permis_sortie" | "permis" | "sortie" => Some(Self::PermisSortie),
            "retard" => Some(Self::Retard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresenceInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub affectation_id: Uuid,
    pub jour: NaiveDate,
    pub statut: PresenceInternatStatut,
    pub note: String,
    pub actor_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PointerPresenceInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub statut: String,
    pub note: String,
    pub actor_user_id: Uuid,
}

pub fn plan_pointer_presence_internat(
    cmd: PointerPresenceInternat,
    affectation: &AffectationInternat,
) -> Result<PresenceInternat, DomainError> {
    if cmd.etablissement_id != affectation.etablissement_id
        || cmd.eleve_id != affectation.eleve_id
    {
        return Err(DomainError::TenantMismatch);
    }
    if !affectation.est_active_au(cmd.jour) {
        return Err(DomainError::InternatAffectationInvalide);
    }
    let statut =
        PresenceInternatStatut::parse(&cmd.statut).ok_or(DomainError::InternatPresenceInvalide)?;
    let note = cmd.note.trim().to_string();
    if note.len() > 500 {
        return Err(DomainError::InternatPresenceInvalide);
    }
    Ok(PresenceInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        affectation_id: affectation.id,
        jour: cmd.jour,
        statut,
        note,
        actor_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiviInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub texte: String,
    pub visible_famille: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerSuiviInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub texte: String,
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_suivi_internat(cmd: CreerSuiviInternat) -> Result<SuiviInternat, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let texte = cmd.texte.trim().to_string();
    if texte.is_empty() || texte.len() > 2000 {
        return Err(DomainError::InternatSuiviInvalide);
    }
    Ok(SuiviInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        texte,
        visible_famille: cmd.visible_famille,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapInternatEleve {
    pub affectation: Option<AffectationInternat>,
    pub chambre: Option<ChambreInternat>,
    pub presence_du_jour: Option<PresenceInternat>,
    pub suivis_famille: Vec<SuiviInternat>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chambre(cap: i16) -> ChambreInternat {
        ChambreInternat {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            batiment: "A".into(),
            etage: 1,
            code: "A12".into(),
            capacite: cap,
            genre: ChambreGenre::Garcons,
            actif: true,
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn affectation_exige_regime_int_et_capacite() {
        let ch = chambre(1);
        let etab = ch.etablissement_id;
        let eleve = Uuid::new_v4();
        let err = plan_affecter_eleve_internat(
            AffecterEleveInternat {
                etablissement_id: etab,
                eleve_id: eleve,
                chambre_id: ch.id,
                lit: Some("1".into()),
                debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                actor_user_id: Uuid::new_v4(),
            },
            &ch,
            Some("DP"),
            0,
            false,
        )
        .unwrap_err();
        assert_eq!(err, DomainError::InternatRegimeRequis);

        let plein = plan_affecter_eleve_internat(
            AffecterEleveInternat {
                etablissement_id: etab,
                eleve_id: eleve,
                chambre_id: ch.id,
                lit: None,
                debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                actor_user_id: Uuid::new_v4(),
            },
            &ch,
            Some("internat"),
            1,
            false,
        )
        .unwrap_err();
        assert_eq!(plein, DomainError::InternatChambrePleine);

        let ok = plan_affecter_eleve_internat(
            AffecterEleveInternat {
                etablissement_id: etab,
                eleve_id: eleve,
                chambre_id: ch.id,
                lit: Some(" lit 2 ".into()),
                debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                actor_user_id: Uuid::new_v4(),
            },
            &ch,
            Some("INT"),
            0,
            false,
        )
        .unwrap();
        assert_eq!(ok.lit.as_deref(), Some("lit 2"));
        assert!(ok.est_active_au(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()));
    }

    #[test]
    fn presence_refuse_hors_affectation() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let aff = AffectationInternat {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            eleve_id: eleve,
            chambre_id: Uuid::new_v4(),
            lit: None,
            debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            fin: Some(NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()),
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };
        let err = plan_pointer_presence_internat(
            PointerPresenceInternat {
                etablissement_id: etab,
                eleve_id: eleve,
                jour: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
                statut: "present".into(),
                note: String::new(),
                actor_user_id: Uuid::new_v4(),
            },
            &aff,
        )
        .unwrap_err();
        assert_eq!(err, DomainError::InternatAffectationInvalide);
    }
}
