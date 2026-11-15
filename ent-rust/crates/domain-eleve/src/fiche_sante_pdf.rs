//! Résumé PDF fiche santé (face famille) — allergies / traitements / PAI / urgence.
//! Jamais la note confidentielle. Pas de PAI interactif ici.

use crate::{DomainError, FicheSante};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeFicheSante {
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub classe: Option<String>,
    pub allergies: String,
    pub traitements: String,
    pub contre_indications: String,
    pub pai_actif: bool,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
    pub date_document: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct GenererResumeFicheSante {
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub classe: Option<String>,
    pub date_document: NaiveDate,
}

pub fn plan_resume_fiche_sante(
    fiche: &FicheSante,
    cmd: GenererResumeFicheSante,
) -> Result<ResumeFicheSante, DomainError> {
    let nom = cmd.eleve_nom.trim().to_string();
    let prenom = cmd.eleve_prenom.trim().to_string();
    if nom.is_empty() || prenom.is_empty() {
        return Err(DomainError::FicheSanteInvalide);
    }
    let allergies = fiche.allergies.trim().to_string();
    let traitements = fiche.traitements.trim().to_string();
    let contre = fiche.contre_indications.trim().to_string();
    if allergies.is_empty()
        && traitements.is_empty()
        && contre.is_empty()
        && !fiche.pai_actif
        && fiche
            .medecin_nom
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        && fiche
            .urgence_tel
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(DomainError::FicheSanteInvalide);
    }
    Ok(ResumeFicheSante {
        eleve_nom: nom,
        eleve_prenom: prenom,
        classe: cmd
            .classe
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty()),
        allergies,
        traitements,
        contre_indications: contre,
        pai_actif: fiche.pai_actif,
        medecin_nom: fiche.medecin_nom.clone(),
        urgence_tel: fiche.urgence_tel.clone(),
        date_document: cmd.date_document,
    })
}

/// PDF résumé santé — champs famille uniquement.
pub fn render_resume_fiche_sante_pdf(r: &ResumeFicheSante) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Résumé santé élève".into(),
        String::new(),
        format!("Date : {}", r.date_document),
        format!("Élève : {} {}", r.eleve_prenom, r.eleve_nom),
    ];
    if let Some(cl) = &r.classe {
        lines.push(format!("Classe : {cl}"));
    }
    lines.push(format!(
        "PAI : {}",
        if r.pai_actif { "actif" } else { "non" }
    ));
    lines.push(String::new());
    lines.push(format!(
        "Allergies : {}",
        if r.allergies.is_empty() {
            "—"
        } else {
            &r.allergies
        }
    ));
    lines.push(format!(
        "Traitements : {}",
        if r.traitements.is_empty() {
            "—"
        } else {
            &r.traitements
        }
    ));
    lines.push(format!(
        "Contre-indications : {}",
        if r.contre_indications.is_empty() {
            "—"
        } else {
            &r.contre_indications
        }
    ));
    if let Some(m) = &r.medecin_nom {
        lines.push(format!("Médecin : {m}"));
    }
    if let Some(t) = &r.urgence_tel {
        lines.push(format!("Urgence : {t}"));
    }
    lines.push(String::new());
    lines.push("Document famille — aucune note infirmiere confidentielle.".into());
    crate::pdf_commun::rendre_lignes("Résumé santé élève", Some(r.date_document), &lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_upsert_fiche_sante, UpsertFicheSante};
    use uuid::Uuid;

    #[test]
    fn pdf_masque_note() {
        let f = plan_upsert_fiche_sante(UpsertFicheSante {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            allergies: "arachides".into(),
            traitements: "Ventoline".into(),
            contre_indications: String::new(),
            pai_actif: true,
            medecin_nom: Some("Dr Pomfrey".into()),
            urgence_tel: Some("0600000000".into()),
            note_confidentielle: "secret clinique asthme".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let r = plan_resume_fiche_sante(
            &f,
            GenererResumeFicheSante {
                eleve_nom: "Potter".into(),
                eleve_prenom: "Harry".into(),
                classe: Some("4B".into()),
                date_document: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            },
        )
        .unwrap();
        let pdf = render_resume_fiche_sante_pdf(&r);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        // Police incorporée : le texte se relit par la ToUnicode, pas dans les octets bruts.
        let s = ent_pdf::releve::texte(&pdf);
        assert!(s.contains("arachides"));
        assert!(!s.contains("secret clinique"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }

    #[test]
    fn fiche_sante_accents_lisibles() {
        use ent_pdf::releve::{texte, verifier_structure};
        let f = plan_upsert_fiche_sante(UpsertFicheSante {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            allergies: "œufs, fruits à coque, noisettes".into(),
            traitements: "Ventoline — 2 bouffées si crise".into(),
            contre_indications: "Pas d'effort prolongé à l'extérieur".into(),
            pai_actif: true,
            medecin_nom: Some("Dr Hélène Müller".into()),
            urgence_tel: Some("0600000000".into()),
            note_confidentielle: "secret clinique".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let r = plan_resume_fiche_sante(
            &f,
            GenererResumeFicheSante {
                eleve_nom: "Élève".into(),
                eleve_prenom: "Zoé".into(),
                classe: Some("6ᵉ A".into()),
                date_document: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            },
        )
        .unwrap();
        let pdf = render_resume_fiche_sante_pdf(&r);
        verifier_structure(&pdf);
        let t = texte(&pdf);
        assert!(t.contains("Résumé santé élève"), "{t}");
        assert!(t.contains("Élève : Zoé Élève"));
        assert!(t.contains("Allergies : œufs, fruits à coque, noisettes"));
        assert!(t.contains("Médecin : Dr Hélène Müller"));
        assert!(!t.contains("secret clinique"));
    }
}
