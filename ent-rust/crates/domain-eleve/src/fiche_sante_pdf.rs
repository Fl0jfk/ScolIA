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
        && fiche.medecin_nom.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
        && fiche.urgence_tel.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
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

fn pdf_escape(s: &str) -> String {
    let safe = ascii_latin(s);
    let mut out = String::with_capacity(safe.len());
    for c in safe.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\r' | '\n' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

fn ascii_latin(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'À' | 'Á' | 'Â' | 'Ä' => 'a',
            'ç' | 'Ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ñ' | 'Ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ÿ' | 'Ÿ' => 'y',
            'œ' | 'Œ' => 'o',
            '€' => 'E',
            c if c.is_ascii() => c,
            _ => '?',
        })
        .collect()
}

fn build_pdf(lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT\n/F1 11 Tf\n50 800 Td\n14 TL\n");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            content.push_str("T*\n");
        }
        content.push_str(&format!("({}) Tj\n", pdf_escape(line)));
    }
    content.push_str("ET\n");
    let stream = content.into_bytes();

    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
/Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut contents_obj = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    contents_obj.extend_from_slice(&stream);
    contents_obj.extend_from_slice(b"\nendstream");
    objects.push(contents_obj);
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0);
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// PDF résumé santé — champs famille uniquement.
pub fn render_resume_fiche_sante_pdf(r: &ResumeFicheSante) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Resume sante eleve".into(),
        String::new(),
        format!("Date : {}", r.date_document),
        format!("Eleve : {} {}", r.eleve_prenom, r.eleve_nom),
    ];
    if let Some(cl) = &r.classe {
        lines.push(format!("Classe : {cl}"));
    }
    lines.push(format!(
        "PAI   : {}",
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
        lines.push(format!("Medecin : {m}"));
    }
    if let Some(t) = &r.urgence_tel {
        lines.push(format!("Urgence : {t}"));
    }
    lines.push(String::new());
    lines.push("Document famille — aucune note infirmiere confidentielle.".into());
    build_pdf(&lines)
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
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("arachides"));
        assert!(!s.contains("secret clinique"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }
}
