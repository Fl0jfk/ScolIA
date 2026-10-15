//! Export Notanet / notes bac (P2 stub CSV).

use crate::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LigneNotanet {
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub matiere: String,
    pub note_sur_20: f64,
}

pub fn render_notanet_csv(lignes: &[LigneNotanet]) -> String {
    let mut out = String::from("ine;nom;prenom;matiere;note\n");
    for l in lignes {
        out.push_str(&format!(
            "{};{};{};{};{:.2}\n",
            esc(&l.ine),
            esc(&l.nom),
            esc(&l.prenom),
            esc(&l.matiere),
            l.note_sur_20
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
pub struct ExporterNotanet {
    pub etablissement_id: Uuid,
    pub periode_id: Uuid,
    pub classe: String,
}

pub fn plan_exporter_notanet(cmd: ExporterNotanet) -> Result<ExporterNotanet, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.periode_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.classe.trim().is_empty() {
        return Err(DomainError::NotFound);
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv() {
        assert!(render_notanet_csv(&[]).starts_with("ine;nom;"));
    }
}
