//! Export ONDE / BE1D — CSV minimal 1D (P2 stub).

use crate::DomainError;
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LigneOnde {
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: String,
    pub sexe: String,
    pub classe: String,
    pub mef: String,
}

pub fn render_onde_csv(lignes: &[LigneOnde]) -> String {
    let mut out = String::from("ine;nom;prenom;date_naissance;sexe;classe;mef\n");
    for l in lignes {
        out.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            esc(&l.ine),
            esc(&l.nom),
            esc(&l.prenom),
            esc(&l.date_naissance),
            esc(&l.sexe),
            esc(&l.classe),
            esc(&l.mef),
        ));
    }
    out
}

fn esc(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ExporterOnde {
    pub etablissement_id: Uuid,
    pub classe: Option<String>,
    pub jour_reference: NaiveDate,
}

pub fn plan_exporter_onde(cmd: ExporterOnde) -> Result<ExporterOnde, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let csv = render_onde_csv(&[]);
        assert!(csv.starts_with("ine;nom;"));
    }
}
