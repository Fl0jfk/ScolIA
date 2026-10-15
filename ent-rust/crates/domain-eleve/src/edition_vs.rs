//! Éditions / documents VS — modèles paramétrables + rendu texte (P2).

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionVsTemplate {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub titre: String,
    /// Corps avec placeholders `{{nom}}`, `{{prenom}}`, `{{classe}}`, `{{date}}`.
    pub corps: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerEditionVs {
    pub etablissement_id: Uuid,
    pub code: String,
    pub titre: String,
    pub corps: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_edition_vs(cmd: CreerEditionVs) -> Result<EditionVsTemplate, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_ascii_uppercase();
    if code.is_empty() || code.len() > 40 {
        return Err(DomainError::EditionVsInvalide);
    }
    let titre = cmd.titre.trim().to_string();
    if titre.is_empty() || titre.len() > 200 {
        return Err(DomainError::EditionVsInvalide);
    }
    let corps = cmd.corps.trim().to_string();
    if corps.is_empty() || corps.len() > 20_000 {
        return Err(DomainError::EditionVsInvalide);
    }
    Ok(EditionVsTemplate {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        titre,
        corps,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone)]
pub struct RendreEditionVs {
    pub nom: String,
    pub prenom: String,
    pub classe: String,
    pub date: String,
}

/// Remplace `{{cle}}` dans le corps (clés connues uniquement).
pub fn rendre_edition_vs(corps: &str, vars: &RendreEditionVs) -> String {
    corps
        .replace("{{nom}}", &vars.nom)
        .replace("{{prenom}}", &vars.prenom)
        .replace("{{classe}}", &vars.classe)
        .replace("{{date}}", &vars.date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendu_placeholders() {
        let out = rendre_edition_vs(
            "Convocation {{prenom}} {{nom}} ({{classe}}) le {{date}}",
            &RendreEditionVs {
                nom: "POTTER".into(),
                prenom: "Harry".into(),
                classe: "4B".into(),
                date: "2026-10-14".into(),
            },
        );
        assert!(out.contains("Harry POTTER"));
        assert!(out.contains("4B"));
    }

    #[test]
    fn code_vide_refuse() {
        let err = plan_creer_edition_vs(CreerEditionVs {
            etablissement_id: Uuid::new_v4(),
            code: "  ".into(),
            titre: "Convoc".into(),
            corps: "Bonjour {{prenom}}".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EditionVsInvalide);
    }
}
