//! Bulletin PDF minimal — invariant : période **fermée** uniquement.
//!
//! Templates stylés catalogue = profondeur ultérieure ; ici = geste métier
//! observable (période clôturée → PDF → dépôt docs famille).

use crate::{
    Appreciation, DomainError, MoyenneMatiere, NotePeriode, NotePeriodeStatut,
};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct LigneBulletinMatiere {
    pub matiere_libelle: String,
    pub moyenne_sur_20: f64,
    pub appreciation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BulletinEleve {
    pub eleve_prenom: String,
    pub eleve_nom: String,
    pub classe: Option<String>,
    pub periode_libelle: String,
    pub periode_code: String,
    pub date_document: NaiveDate,
    pub lignes: Vec<LigneBulletinMatiere>,
    pub moyenne_generale: Option<f64>,
    pub appreciation_generale: Option<String>,
}

/// Invariant produit : bulletin imprimable seulement si période clôturée.
pub fn assert_periode_bulletin(periode: &NotePeriode) -> Result<(), DomainError> {
    if periode.statut != NotePeriodeStatut::Fermee {
        return Err(DomainError::NoteBulletinPeriodeNonFermee);
    }
    Ok(())
}

pub fn plan_bulletin_eleve(
    periode: &NotePeriode,
    eleve_prenom: &str,
    eleve_nom: &str,
    classe: Option<&str>,
    moyennes: &[(MoyenneMatiere, String)],
    appreciations: &[Appreciation],
    date_document: NaiveDate,
) -> Result<BulletinEleve, DomainError> {
    assert_periode_bulletin(periode)?;
    let app_gen = appreciations
        .iter()
        .find(|a| a.matiere_id.is_none())
        .map(|a| a.texte.clone());
    let mut lignes = Vec::new();
    let mut sum = 0.0;
    let mut n = 0usize;
    for (m, lib) in moyennes {
        let app = appreciations
            .iter()
            .find(|a| a.matiere_id == Some(m.matiere_id))
            .map(|a| a.texte.clone());
        sum += m.moyenne_sur_20;
        n += 1;
        lignes.push(LigneBulletinMatiere {
            matiere_libelle: lib.clone(),
            moyenne_sur_20: m.moyenne_sur_20,
            appreciation: app,
        });
    }
    lignes.sort_by(|a, b| a.matiere_libelle.cmp(&b.matiere_libelle));
    let moyenne_generale = if n == 0 { None } else { Some(sum / n as f64) };
    Ok(BulletinEleve {
        eleve_prenom: eleve_prenom.trim().to_string(),
        eleve_nom: eleve_nom.trim().to_string(),
        classe: classe.map(|s| s.to_string()),
        periode_libelle: periode.libelle.clone(),
        periode_code: periode.code.clone(),
        date_document,
        lignes,
        moyenne_generale,
        appreciation_generale: app_gen,
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

pub(crate) fn build_pdf(lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT\n/F1 11 Tf\n50 800 Td\n13 TL\n");
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

pub fn render_bulletin_pdf(b: &BulletinEleve) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Bulletin scolaire".into(),
        String::new(),
        format!("Date : {}", b.date_document),
        format!(
            "Eleve : {} {} · Classe : {}",
            b.eleve_prenom,
            b.eleve_nom,
            b.classe.as_deref().unwrap_or("—")
        ),
        format!("Periode : {} ({})", b.periode_libelle, b.periode_code),
        String::new(),
        "Moyennes /20".into(),
        "-------------".into(),
    ];
    for l in &b.lignes {
        lines.push(format!(
            "{} : {:.2}",
            l.matiere_libelle, l.moyenne_sur_20
        ));
        if let Some(app) = &l.appreciation {
            lines.push(format!("  Apprec. : {app}"));
        }
    }
    if let Some(mg) = b.moyenne_generale {
        lines.push(String::new());
        lines.push(format!("Moyenne generale : {mg:.2} / 20"));
    }
    if let Some(ag) = &b.appreciation_generale {
        lines.push(String::new());
        lines.push(format!("Appreciation generale : {ag}"));
    }
    lines.push(String::new());
    lines.push("Document publie dans l'espace documents (famille).".into());
    build_pdf(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NotePeriodeStatut;
    use uuid::Uuid;

    fn periode(statut: NotePeriodeStatut) -> NotePeriode {
        NotePeriode {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            code: "T1".into(),
            libelle: "Trimestre 1".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: NaiveDate::from_ymd_opt(2026, 12, 20).unwrap(),
            statut,
        }
    }

    #[test]
    fn refuse_si_ouverte() {
        let err = assert_periode_bulletin(&periode(NotePeriodeStatut::Ouverte)).unwrap_err();
        assert_eq!(err, DomainError::NoteBulletinPeriodeNonFermee);
    }

    #[test]
    fn pdf_si_fermee() {
        let p = periode(NotePeriodeStatut::Fermee);
        let mid = Uuid::new_v4();
        let b = plan_bulletin_eleve(
            &p,
            "Harry",
            "Potter",
            Some("4B"),
            &[(
                MoyenneMatiere {
                    matiere_id: mid,
                    periode_id: p.id,
                    moyenne_sur_20: 14.5,
                },
                "Maths".into(),
            )],
            &[],
            NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
        )
        .unwrap();
        let pdf = render_bulletin_pdf(&b);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }
}
