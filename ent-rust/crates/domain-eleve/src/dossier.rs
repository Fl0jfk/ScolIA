//! Dossier élève — agrégat de lecture + options + invariant payeurs.
//!
//! Le dossier n'est **pas** une fiche « toujours vraie depuis toujours » :
//! identité + projections courantes + historique daté + encart foyers + options.
//! Facturation reste sur le foyer (pas un onglet central ici).

use crate::{
    normalize_ine, CarnetEntree, DomainError, Eleve, EleveBourse, EleveDispenseRepas,
    EleveDispositif, EleveDocument, EleveGrilleRepas, ElevePeriode, Incident, OptionAnnee, Sanction,
    ScolariteAnnee,
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionNature {
    Academique,
    Interne,
}

impl OptionNature {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Academique => "academique",
            Self::Interne => "interne",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "academique" => Some(Self::Academique),
            "interne" => Some(Self::Interne),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveOption {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nature: OptionNature,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct AjouterOption {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nature: OptionNature,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateIdentite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub ine: Option<Option<String>>,
    pub sexe: Option<Option<String>>,
    pub date_naissance: Option<Option<NaiveDate>>,
    pub numero_badge: Option<Option<String>>,
    pub redoublant: Option<bool>,
    pub photo_key: Option<Option<String>>,
    /// Date d’entrée établissement (identité administrative — pas une période scolarité).
    pub date_entree: Option<Option<NaiveDate>>,
    pub actor_user_id: Option<String>,
}

/// Résumé responsable sur la fiche (encart foyer).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LienDossier {
    pub lien_id: Uuid,
    pub user_id: Uuid,
    pub foyer_id: Option<Uuid>,
    pub foyer_libelle: Option<String>,
    pub lien: String,
    pub display_name: Option<String>,
    /// Téléphone du responsable (si renseigné sur le compte).
    pub telephone: Option<String>,
    /// E-mail du responsable (si renseigné sur le compte).
    pub email: Option<String>,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

/// Ce que l'acteur a le droit de voir sur ce dossier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DossierSections {
    pub identite: bool,
    pub scolarite: bool,
    pub famille: bool,
    pub options: bool,
    pub notes: bool,
    pub vs: bool,
    pub documents: bool,
    pub dispositifs: bool,
    pub repas: bool,
    pub bourses: bool,
    /// Continuité N-1 / N / N+1 sur la fiche (pas de year-jump).
    pub continuum: bool,
}

/// Vue dossier élève (lecture) — sections masquées selon authz.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DossierEleve {
    pub eleve: Eleve,
    pub age_ans: Option<u32>,
    pub periodes: Vec<ElevePeriode>,
    pub options: Vec<EleveOption>,
    pub liens: Vec<LienDossier>,
    /// Documents **visibles pour cet acteur** (déjà filtrés).
    pub documents: Vec<EleveDocument>,
    pub dispositifs: Vec<EleveDispositif>,
    pub grille_repas: Vec<EleveGrilleRepas>,
    /// Dispenses / prévenances actives (hors annulation).
    pub dispenses_repas: Vec<EleveDispenseRepas>,
    pub bourses: Vec<EleveBourse>,
    /// Snapshots N-1 / N / N+1 visibles pour l'acteur.
    pub continuum: Vec<ScolariteAnnee>,
    pub options_continuum: Vec<OptionAnnee>,
    /// Sanctions / incidents / carnet visibles (section VS — aussi sur fiche).
    pub sanctions: Vec<Sanction>,
    pub incidents: Vec<Incident>,
    pub carnet: Vec<CarnetEntree>,
    /// Notes visibles (section notes) — sans auteur.
    pub notes: Vec<crate::NoteLue>,
    pub sections: DossierSections,
}

pub fn age_ans_au(date_naissance: NaiveDate, jour: NaiveDate) -> u32 {
    let mut age = jour.year() - date_naissance.year();
    if (jour.month(), jour.day()) < (date_naissance.month(), date_naissance.day()) {
        age -= 1;
    }
    u32::try_from(age.max(0)).unwrap_or(0)
}

/// Invariant dur : somme des parts des payeurs **par élève** = 100 %, ou aucun payeur.
pub fn assert_parts_paiement(
    parts: &[(bool, Option<i16>)],
) -> Result<(), DomainError> {
    let payeurs: Vec<i16> = parts
        .iter()
        .filter(|(est, _)| *est)
        .map(|(_, p)| p.unwrap_or(0))
        .collect();
    if payeurs.is_empty() {
        return Ok(());
    }
    let sum: i32 = payeurs.iter().map(|p| i32::from(*p)).sum();
    if sum != 100 {
        return Err(DomainError::PartsPaiementInvalides);
    }
    Ok(())
}

pub fn plan_ajouter_option(cmd: AjouterOption) -> Result<EleveOption, DomainError> {
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
    Ok(EleveOption {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        nature: cmd.nature,
        code,
        libelle,
        date_debut: cmd.date_debut,
        date_fin: None,
    })
}

pub fn plan_update_identite(
    current: &Eleve,
    cmd: UpdateIdentite,
) -> Result<(Eleve, bool), DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.eleve_id {
        return Err(DomainError::NotFound);
    }
    let mut eleve = current.clone();
    let mut changed = false;

    if let Some(nom) = cmd.nom {
        let nom = nom.trim().to_string();
        if nom.is_empty() {
            return Err(DomainError::IdentityRequired);
        }
        if eleve.nom != nom {
            eleve.nom = nom;
            changed = true;
        }
    }
    if let Some(prenom) = cmd.prenom {
        let prenom = prenom.trim().to_string();
        if prenom.is_empty() {
            return Err(DomainError::IdentityRequired);
        }
        if eleve.prenom != prenom {
            eleve.prenom = prenom;
            changed = true;
        }
    }
    if let Some(ine) = cmd.ine {
        let ine = match ine {
            Some(raw) if !raw.trim().is_empty() => Some(normalize_ine(&raw)?),
            Some(_) | None => None,
        };
        if eleve.ine != ine {
            eleve.ine = ine;
            changed = true;
        }
    }
    if let Some(sexe) = cmd.sexe {
        if eleve.sexe != sexe {
            eleve.sexe = sexe;
            changed = true;
        }
    }
    if let Some(dn) = cmd.date_naissance {
        if eleve.date_naissance != dn {
            eleve.date_naissance = dn;
            changed = true;
        }
    }
    if let Some(badge) = cmd.numero_badge {
        if eleve.numero_badge != badge {
            eleve.numero_badge = badge;
            changed = true;
        }
    }
    if let Some(r) = cmd.redoublant {
        if eleve.redoublant != r {
            eleve.redoublant = r;
            changed = true;
        }
    }
    if let Some(photo) = cmd.photo_key {
        if eleve.photo_key != photo {
            eleve.photo_key = photo;
            changed = true;
        }
    }
    if let Some(de) = cmd.date_entree {
        if eleve.date_entree != de {
            eleve.date_entree = de;
            changed = true;
        }
    }

    Ok((eleve, changed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_doivent_faire_100() {
        assert!(assert_parts_paiement(&[(true, Some(60)), (true, Some(40))]).is_ok());
        assert!(assert_parts_paiement(&[(true, Some(50)), (true, Some(40))]).is_err());
        assert!(assert_parts_paiement(&[(false, None), (false, None)]).is_ok());
    }

    #[test]
    fn age_calcule() {
        let n = NaiveDate::from_ymd_opt(2012, 6, 15).unwrap();
        let j = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        assert_eq!(age_ans_au(n, j), 14);
        let avant = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(age_ans_au(n, avant), 13);
    }
}
