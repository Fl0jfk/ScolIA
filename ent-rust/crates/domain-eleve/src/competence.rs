//! Compétences / LSU — socle commun évalué **par établissement**.
//!
//! Deux objets :
//! - `CompetenceItem` = le **référentiel** (domaine D1–D5, code, libellé), propre
//!   à l'établissement (modifiable) — pas de règle nationale figée dans le code.
//! - `EvaluationCompetence` = le **niveau de maîtrise** d'un élève sur un item
//!   pour une période (une évaluation par `(élève, item, période)`).
//!
//! Le LSU ne se devine pas depuis une matière : il se **saisit**. L'export
//! (`lsun_export`) lit ces évaluations réelles ; à défaut, il n'invente rien.

use crate::bulletin_pdf::build_pdf;
use crate::DomainError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Échelle nationale des niveaux de maîtrise (stable depuis 2016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NiveauMaitrise {
    Insuffisante,
    Fragile,
    Satisfaisante,
    TresBonne,
}

impl NiveauMaitrise {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Insuffisante => "insuffisante",
            Self::Fragile => "fragile",
            Self::Satisfaisante => "satisfaisante",
            Self::TresBonne => "tres_bonne",
        }
    }

    /// Code LSU numérique (1 = insuffisante … 4 = très bonne maîtrise).
    pub fn code_lsu(self) -> &'static str {
        match self {
            Self::Insuffisante => "1",
            Self::Fragile => "2",
            Self::Satisfaisante => "3",
            Self::TresBonne => "4",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Insuffisante => "Maîtrise insuffisante",
            Self::Fragile => "Maîtrise fragile",
            Self::Satisfaisante => "Maîtrise satisfaisante",
            Self::TresBonne => "Très bonne maîtrise",
        }
    }

    /// Accepte le mot (`tres_bonne`), la forme lisible (`très bonne maîtrise`) ou le code (`4`).
    pub fn parse(s: &str) -> Option<Self> {
        let norm = s
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '\u{00a0}'], "_");
        match norm.trim_start_matches("maitrise_").trim_start_matches("maîtrise_") {
            "1" | "insuffisante" => Some(Self::Insuffisante),
            "2" | "fragile" => Some(Self::Fragile),
            "3" | "satisfaisante" => Some(Self::Satisfaisante),
            "4" | "tres_bonne" | "très_bonne" | "tresbonne" => Some(Self::TresBonne),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompetenceItem {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    /// Domaine du socle (« D1 »…« D5 » ou code établissement).
    pub domaine: String,
    pub code: String,
    pub libelle: String,
    pub ordre: i32,
    pub actif: bool,
}

#[derive(Debug, Clone)]
pub struct CreerCompetenceItem {
    pub etablissement_id: Uuid,
    pub domaine: String,
    pub code: String,
    pub libelle: String,
    pub ordre: Option<i32>,
}

const DOMAINE_MAX: usize = 16;
const CODE_MAX: usize = 40;
const LIBELLE_MAX: usize = 300;

pub fn plan_creer_competence_item(
    cmd: CreerCompetenceItem,
) -> Result<CompetenceItem, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let domaine = cmd.domaine.trim().to_string();
    let code = cmd.code.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if domaine.is_empty()
        || domaine.chars().count() > DOMAINE_MAX
        || code.is_empty()
        || code.chars().count() > CODE_MAX
        || libelle.is_empty()
        || libelle.chars().count() > LIBELLE_MAX
    {
        return Err(DomainError::CompetenceInvalide);
    }
    Ok(CompetenceItem {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        domaine,
        code,
        libelle,
        ordre: cmd.ordre.unwrap_or(0).clamp(0, 9999),
        actif: true,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationCompetence {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub item_id: Uuid,
    pub periode_id: Uuid,
    pub niveau: NiveauMaitrise,
    pub commentaire: Option<String>,
    pub auteur_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct EvaluerCompetence {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub item_id: Uuid,
    pub periode_id: Uuid,
    pub niveau: String,
    pub commentaire: Option<String>,
    pub actor_user_id: Uuid,
}

const COMMENTAIRE_MAX: usize = 1000;

pub fn plan_evaluer_competence(
    cmd: EvaluerCompetence,
) -> Result<EvaluationCompetence, DomainError> {
    if cmd.etablissement_id.is_nil()
        || cmd.eleve_id.is_nil()
        || cmd.item_id.is_nil()
        || cmd.periode_id.is_nil()
    {
        return Err(DomainError::CompetenceInvalide);
    }
    let niveau = NiveauMaitrise::parse(&cmd.niveau).ok_or(DomainError::CompetenceInvalide)?;
    let commentaire = cmd
        .commentaire
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    if let Some(c) = &commentaire {
        if c.chars().count() > COMMENTAIRE_MAX {
            return Err(DomainError::CompetenceInvalide);
        }
    }
    Ok(EvaluationCompetence {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        item_id: cmd.item_id,
        periode_id: cmd.periode_id,
        niveau,
        commentaire,
        auteur_user_id: cmd.actor_user_id,
    })
}

/// Une ligne du livret de compétences (PDF / affichage famille).
#[derive(Debug, Clone)]
pub struct LigneLivretCompetence {
    pub domaine: String,
    pub item: String,
    pub niveau: Option<NiveauMaitrise>,
    pub commentaire: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LivretCompetences {
    pub eleve_prenom: String,
    pub eleve_nom: String,
    pub classe: Option<String>,
    pub periode_libelle: String,
    pub periode_code: String,
    pub date_document: NaiveDate,
    pub lignes: Vec<LigneLivretCompetence>,
}

/// Rendu PDF minimal du livret (même moteur que le bulletin).
pub fn render_livret_competences_pdf(l: &LivretCompetences) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Livret de compétences".to_string(),
        String::new(),
        format!("Date : {}", l.date_document),
        format!(
            "Eleve : {} {} · Classe : {}",
            l.eleve_prenom,
            l.eleve_nom,
            l.classe.as_deref().unwrap_or("—")
        ),
        format!("Periode : {} ({})", l.periode_libelle, l.periode_code),
        String::new(),
    ];
    let mut domaine_courant = String::new();
    for ligne in &l.lignes {
        if ligne.domaine != domaine_courant {
            domaine_courant = ligne.domaine.clone();
            lines.push(String::new());
            lines.push(format!("-- {} --", domaine_courant));
        }
        let niveau = ligne
            .niveau
            .map(|n| n.libelle().to_string())
            .unwrap_or_else(|| "non évalué".into());
        lines.push(format!("{} : {}", ligne.item, niveau));
        if let Some(c) = &ligne.commentaire {
            lines.push(format!("  Note : {c}"));
        }
    }
    lines.push(String::new());
    lines.push("Document publie dans l'espace documents (famille).".into());
    build_pdf(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_cmd() -> CreerCompetenceItem {
        CreerCompetenceItem {
            etablissement_id: Uuid::new_v4(),
            domaine: "D1".into(),
            code: "D1.1".into(),
            libelle: "Comprendre et s'exprimer à l'oral".into(),
            ordre: None,
        }
    }

    #[test]
    fn cree_un_item_valide() {
        let i = plan_creer_competence_item(item_cmd()).unwrap();
        assert_eq!(i.domaine, "D1");
        assert!(i.actif);
        assert_eq!(i.ordre, 0);
    }

    #[test]
    fn item_invalide_refuse() {
        let mut c = item_cmd();
        c.libelle = "   ".into();
        assert_eq!(
            plan_creer_competence_item(c).unwrap_err(),
            DomainError::CompetenceInvalide
        );
        let mut c = item_cmd();
        c.etablissement_id = Uuid::nil();
        assert!(plan_creer_competence_item(c).is_err());
    }

    #[test]
    fn niveau_maitrise_codes_lsu() {
        assert_eq!(NiveauMaitrise::Insuffisante.code_lsu(), "1");
        assert_eq!(NiveauMaitrise::TresBonne.code_lsu(), "4");
        assert_eq!(NiveauMaitrise::parse("3"), Some(NiveauMaitrise::Satisfaisante));
        assert_eq!(
            NiveauMaitrise::parse("maîtrise fragile"),
            Some(NiveauMaitrise::Fragile)
        );
        assert_eq!(NiveauMaitrise::parse("n'importe quoi"), None);
    }

    #[test]
    fn evalue_avec_niveau_valide_et_commentaire_trim() {
        let e = plan_evaluer_competence(EvaluerCompetence {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            periode_id: Uuid::new_v4(),
            niveau: "4".into(),
            commentaire: Some("  excellent trimestre  ".into()),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(e.niveau, NiveauMaitrise::TresBonne);
        assert_eq!(e.commentaire.as_deref(), Some("excellent trimestre"));
    }

    #[test]
    fn evalue_refuse_niveau_inconnu() {
        assert!(plan_evaluer_competence(EvaluerCompetence {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            periode_id: Uuid::new_v4(),
            niveau: "génial".into(),
            commentaire: None,
            actor_user_id: Uuid::new_v4(),
        })
        .is_err());
    }

    #[test]
    fn pdf_livret_genere() {
        let pdf = render_livret_competences_pdf(&LivretCompetences {
            eleve_prenom: "Harry".into(),
            eleve_nom: "Potter".into(),
            classe: Some("4B".into()),
            periode_libelle: "Trimestre 1".into(),
            periode_code: "T1".into(),
            date_document: NaiveDate::from_ymd_opt(2026, 12, 20).unwrap(),
            lignes: vec![LigneLivretCompetence {
                domaine: "D1".into(),
                item: "Comprendre et s'exprimer à l'oral".into(),
                niveau: Some(NiveauMaitrise::Satisfaisante),
                commentaire: None,
            }],
        });
        assert!(pdf.starts_with(b"%PDF"));
    }
}
