//! Export LSUN / livret socle — CSV local (P1 partiel ; dépôt national hors scope).

use crate::DomainError;
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LigneLsun {
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub classe: String,
    /// Domaine du socle (ex. D1, D2…).
    pub domaine: String,
    pub item: String,
    /// Niveau de maîtrise : MF | ECA | A | PA (ou libre).
    pub niveau: String,
    pub periode: String,
}

pub fn render_lsun_csv(lignes: &[LigneLsun]) -> String {
    let mut out = String::from(
        "ine;nom;prenom;classe;domaine;item;niveau;periode\n",
    );
    for l in lignes {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{}\n",
            csv_esc(&l.ine),
            csv_esc(&l.nom),
            csv_esc(&l.prenom),
            csv_esc(&l.classe),
            csv_esc(&l.domaine),
            csv_esc(&l.item),
            csv_esc(&l.niveau),
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
pub struct ExporterLsun {
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub classe: Option<String>,
    pub jour_reference: NaiveDate,
}

pub fn plan_exporter_lsun(cmd: ExporterLsun) -> Result<ExporterLsun, DomainError> {
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

/// Heuristique locale : première lettre matière → domaine D1–D5.
pub fn domaine_depuis_matiere(matiere: &str) -> String {
    let c = matiere
        .chars()
        .next()
        .map(|ch| ch.to_ascii_uppercase())
        .unwrap_or('X');
    match c {
        'A'..='E' => "D1".into(),
        'F'..='J' => "D2".into(),
        'K'..='O' => "D3".into(),
        'P'..='T' => "D4".into(),
        _ => "D5".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_header() {
        let csv = render_lsun_csv(&[LigneLsun {
            ine: "123".into(),
            nom: "POTTER".into(),
            prenom: "Harry".into(),
            classe: "4B".into(),
            domaine: "D1".into(),
            item: "Lire".into(),
            niveau: "MF".into(),
            periode: "T1".into(),
        }]);
        assert!(csv.starts_with("ine;nom;prenom"));
        assert!(csv.contains("D1;Lire;MF"));
    }
}
