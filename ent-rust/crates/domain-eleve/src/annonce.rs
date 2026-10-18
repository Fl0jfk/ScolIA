//! Annonces de l'établissement — diffusion direction → familles / personnels.
//!
//! Une annonce est un geste **de communication pure** : elle n'écrit que sa
//! propre table, puis le service `publier_annonce_for_actor` projette les
//! répercussions transverses (notification mail + push des familles ciblées,
//! événement métier, journal).
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// Public visé par une annonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudienceAnnonce {
    /// Toute la communauté (familles + personnels).
    Tous,
    /// Les familles de l'établissement.
    Parents,
    /// Les personnels uniquement (pas de diffusion famille).
    Personnels,
    /// Les familles d'une classe précise.
    Classe,
}

impl AudienceAnnonce {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tous => "tous",
            Self::Parents => "parents",
            Self::Personnels => "personnels",
            Self::Classe => "classe",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tous" | "tout" => Some(Self::Tous),
            "parents" | "familles" => Some(Self::Parents),
            "personnels" | "staff" => Some(Self::Personnels),
            "classe" => Some(Self::Classe),
            _ => None,
        }
    }

    /// Une annonce destinée aux personnels ne part pas vers les familles.
    pub fn touche_parents(self) -> bool {
        matches!(self, Self::Tous | Self::Parents | Self::Classe)
    }
}

#[derive(Debug, Clone)]
pub struct Annonce {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub titre: String,
    pub corps: String,
    pub audience: AudienceAnnonce,
    /// Classe cible — renseignée uniquement pour `AudienceAnnonce::Classe`.
    pub cible_classe: Option<String>,
    pub auteur_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl Annonce {
    /// Visibilité famille : « tous » et « parents » sont visibles par toutes les
    /// familles ; « classe » seulement par la classe ciblée ; « personnels »
    /// jamais.
    pub fn visible_famille(&self, classe: Option<&str>) -> bool {
        match self.audience {
            AudienceAnnonce::Tous | AudienceAnnonce::Parents => true,
            AudienceAnnonce::Classe => {
                matches!((classe, self.cible_classe.as_deref()), (Some(c), Some(t)) if c == t)
            }
            AudienceAnnonce::Personnels => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreerAnnonce {
    pub etablissement_id: Uuid,
    pub titre: String,
    pub corps: String,
    pub audience: String,
    pub cible_classe: Option<String>,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_annonce(cmd: CreerAnnonce) -> Result<Annonce, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::TenantMismatch);
    }
    let titre = cmd.titre.trim().to_string();
    let corps = cmd.corps.trim().to_string();
    if titre.is_empty() || titre.len() > 200 {
        return Err(DomainError::AnnonceInvalide);
    }
    if corps.is_empty() || corps.len() > 10_000 {
        return Err(DomainError::AnnonceInvalide);
    }
    let audience = AudienceAnnonce::parse(&cmd.audience).ok_or(DomainError::AnnonceInvalide)?;
    let cible_classe = cmd
        .cible_classe
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    if audience == AudienceAnnonce::Classe && cible_classe.is_none() {
        return Err(DomainError::AnnonceInvalide);
    }
    Ok(Annonce {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        titre,
        corps,
        audience,
        cible_classe: if audience == AudienceAnnonce::Classe {
            cible_classe
        } else {
            None
        },
        auteur_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(audience: &str, classe: Option<&str>) -> CreerAnnonce {
        CreerAnnonce {
            etablissement_id: Uuid::new_v4(),
            titre: "Sortie pédagogique".into(),
            corps: "Rendez-vous 8h devant le portail.".into(),
            audience: audience.into(),
            cible_classe: classe.map(str::to_string),
            actor_user_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn audience_classe_exige_une_classe() {
        assert!(plan_creer_annonce(cmd("classe", None)).is_err());
        let a = plan_creer_annonce(cmd("classe", Some("4B"))).unwrap();
        assert_eq!(a.cible_classe.as_deref(), Some("4B"));
        assert!(a.visible_famille(Some("4B")));
        assert!(!a.visible_famille(Some("5A")));
        assert!(!a.visible_famille(None));
    }

    #[test]
    fn titre_ou_corps_vide_refuse() {
        let mut c = cmd("tous", None);
        c.titre = "   ".into();
        assert!(plan_creer_annonce(c).is_err());
        let mut c = cmd("tous", None);
        c.corps = String::new();
        assert!(plan_creer_annonce(c).is_err());
    }

    #[test]
    fn audience_personnels_ne_touche_pas_les_familles() {
        let a = plan_creer_annonce(cmd("personnels", None)).unwrap();
        assert!(!a.audience.touche_parents());
        assert!(!a.visible_famille(Some("4B")));
    }

    #[test]
    fn audience_tous_ignore_la_classe_fournie() {
        let a = plan_creer_annonce(cmd("tous", Some("4B"))).unwrap();
        assert!(a.cible_classe.is_none());
        assert!(a.visible_famille(Some("3C")));
    }
}
