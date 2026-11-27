//! Export Parcoursup / APB — CSV local (P2 ; dépôt national hors scope).

use crate::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LigneParcoursup {
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub classe: String,
    pub formation_souhaitee: String,
    pub rang_voeu: i32,
    pub moyenne_generale: Option<f64>,
}

pub fn render_parcoursup_csv(lignes: &[LigneParcoursup]) -> String {
    let mut out = String::from(
        "ine;nom;prenom;classe;formation;rang_voeu;moyenne\n",
    );
    for l in lignes {
        let moy = l
            .moyenne_generale
            .map(|n| format!("{n:.2}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            esc(&l.ine),
            esc(&l.nom),
            esc(&l.prenom),
            esc(&l.classe),
            esc(&l.formation_souhaitee),
            l.rang_voeu,
            moy,
        ));
    }
    out
}

fn esc(s: &str) -> String {
    if s.contains(';') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ExporterParcoursup {
    pub etablissement_id: Uuid,
    pub annee_scolaire: String,
    pub classe: String,
}

pub fn plan_exporter_parcoursup(
    cmd: ExporterParcoursup,
) -> Result<ExporterParcoursup, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.classe.trim().is_empty() || cmd.annee_scolaire.trim().is_empty() {
        return Err(DomainError::NotFound);
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_header() {
        assert!(render_parcoursup_csv(&[]).starts_with("ine;nom;"));
    }
}
