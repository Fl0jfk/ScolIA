//! Courrier d'orientation psy — PDF minimal (mêmes primitives que facturation_pdf).
//!
//! Contenu **famille** uniquement : identité + axes de suivi. Jamais note / contacts.

use crate::{DomainError, DossierPsy};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourrierOrientationPsy {
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub classe: Option<String>,
    pub axes_suivi: String,
    pub suivi_actif: bool,
    pub date_courrier: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct GenererCourrierOrientationPsy {
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub classe: Option<String>,
    pub date_courrier: NaiveDate,
}

/// Construit le courrier depuis le dossier (axes obligatoires ; jamais note/contacts).
pub fn plan_courrier_orientation_psy(
    dossier: &DossierPsy,
    cmd: GenererCourrierOrientationPsy,
) -> Result<CourrierOrientationPsy, DomainError> {
    let nom = cmd.eleve_nom.trim().to_string();
    let prenom = cmd.eleve_prenom.trim().to_string();
    if nom.is_empty() || prenom.is_empty() {
        return Err(DomainError::CourrierPsyInvalide);
    }
    let axes = dossier.axes_suivi.trim().to_string();
    if axes.is_empty() {
        return Err(DomainError::CourrierPsyInvalide);
    }
    if axes.len() > 4000 {
        return Err(DomainError::CourrierPsyInvalide);
    }
    let classe = cmd
        .classe
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    Ok(CourrierOrientationPsy {
        eleve_nom: nom,
        eleve_prenom: prenom,
        classe,
        axes_suivi: axes,
        suivi_actif: dossier.suivi_actif,
        date_courrier: cmd.date_courrier,
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

/// PDF courrier orientation — axes visibles famille uniquement.
pub fn render_courrier_orientation_pdf(c: &CourrierOrientationPsy) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Courrier orientation (psychologie scolaire)".into(),
        String::new(),
        format!("Date : {}", c.date_courrier),
        format!("Eleve : {} {}", c.eleve_prenom, c.eleve_nom),
    ];
    if let Some(cl) = &c.classe {
        lines.push(format!("Classe : {cl}"));
    }
    lines.push(format!(
        "Suivi  : {}",
        if c.suivi_actif { "actif" } else { "clos" }
    ));
    lines.push(String::new());
    lines.push("Axes de suivi partages avec la famille".into());
    lines.push("--------------------------------------".into());
    for chunk in wrap_axes(&c.axes_suivi, 72) {
        lines.push(chunk);
    }
    lines.push(String::new());
    lines.push(
        "Document informatif — contenu partageable famille uniquement.".into(),
    );
    build_pdf(&lines)
}

fn wrap_axes(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if line.is_empty() {
            line = word.to_string();
        } else if line.len() + 1 + word.len() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_upsert_dossier_psy, UpsertDossierPsy};
    use uuid::Uuid;

    fn dossier_ok() -> DossierPsy {
        plan_upsert_dossier_psy(UpsertDossierPsy {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            axes_suivi: "anxiété scolaire · orientation 3e".into(),
            contacts_externes: "CMP secret".into(),
            note_confidentielle: "note clinique".into(),
            suivi_actif: true,
            signale_famille: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap()
    }

    #[test]
    fn refuse_sans_axes() {
        let mut d = dossier_ok();
        d.axes_suivi = String::new();
        let err = plan_courrier_orientation_psy(
            &d,
            GenererCourrierOrientationPsy {
                eleve_nom: "Potter".into(),
                eleve_prenom: "Harry".into(),
                classe: Some("4B".into()),
                date_courrier: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::CourrierPsyInvalide);
    }

    #[test]
    fn pdf_sans_note_ni_contacts() {
        let d = dossier_ok();
        let c = plan_courrier_orientation_psy(
            &d,
            GenererCourrierOrientationPsy {
                eleve_nom: "Potter".into(),
                eleve_prenom: "Harry".into(),
                classe: Some("4B".into()),
                date_courrier: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            },
        )
        .unwrap();
        let pdf = render_courrier_orientation_pdf(&c);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("orientation 3e") || s.contains("orientation"));
        assert!(!s.contains("CMP secret"));
        assert!(!s.contains("hypothese") && !s.contains("note clinique"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }
}
