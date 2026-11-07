//! Export LSL / notes nationales — CSV minimal (P1 partiel).

use crate::DomainError;
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LigneLsl {
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub classe: String,
    pub matiere_code: String,
    pub note_sur_20: Option<f64>,
    pub appreciation: String,
    pub periode: String,
}

pub fn render_lsl_csv(lignes: &[LigneLsl]) -> String {
    let mut out = String::from(
        "ine;nom;prenom;classe;matiere;note_20;appreciation;periode\n",
    );
    for l in lignes {
        let note = l
            .note_sur_20
            .map(|n| format!("{n:.2}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{}\n",
            csv_esc(&l.ine),
            csv_esc(&l.nom),
            csv_esc(&l.prenom),
            csv_esc(&l.classe),
            csv_esc(&l.matiere_code),
            note,
            csv_esc(&l.appreciation),
            csv_esc(&l.periode),
        ));
    }
    out
}

fn csv_esc(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ExporterLsl {
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub classe: Option<String>,
    pub jour_reference: NaiveDate,
}

pub fn plan_exporter_lsl(cmd: ExporterLsl) -> Result<ExporterLsl, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.periode_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd
        .classe
        .as_ref()
        .map(|c| c.trim().is_empty())
        .unwrap_or(true)
    {
        return Err(DomainError::NotFound);
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_header() {
        let csv = render_lsl_csv(&[LigneLsl {
            ine: "AB1234567".into(),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            classe: "4B".into(),
            matiere_code: "MATH".into(),
            note_sur_20: Some(14.5),
            appreciation: "Bon travail".into(),
            periode: "T1".into(),
        }]);
        assert!(csv.starts_with("ine;nom;"));
        assert!(csv.contains("14.50"));
    }
}
