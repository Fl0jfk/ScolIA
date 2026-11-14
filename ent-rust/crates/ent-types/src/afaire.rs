//! Socle 29, lots P2 et P3 : l'« à-faire » unifié (`docs/socles/29_PORTAILS.md` §8).
//!
//! Types d'**échange** de l'API (D-036 : le JSON est ici un format de transport, jamais une
//! mémoire). L'index est une projection : le titre est **neutre** (R-PO5), sans motif, sans montant,
//! sans libellé de santé ni de discipline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priorité d'une tâche (§7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrioriteAfaire {
    Bloquante,
    Due,
    AFaire,
    Information,
}

/// État d'une tâche (§8.4). Les états finaux ne sont jamais montrés comme « à faire ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatAfaire {
    Ouverte,
    Prise,
    EnAttente,
    Faite,
    SansObjet,
    Expiree,
    Annulee,
}

impl EtatAfaire {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ouverte => "ouverte",
            Self::Prise => "prise",
            Self::EnAttente => "en_attente",
            Self::Faite => "faite",
            Self::SansObjet => "sans_objet",
            Self::Expiree => "expiree",
            Self::Annulee => "annulee",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ouverte" => Self::Ouverte,
            "prise" => Self::Prise,
            "en_attente" => Self::EnAttente,
            "faite" => Self::Faite,
            "sans_objet" => Self::SansObjet,
            "expiree" => Self::Expiree,
            "annulee" => Self::Annulee,
            _ => return None,
        })
    }
    /// Une tâche encore à traiter.
    pub fn est_ouvert(self) -> bool {
        matches!(self, Self::Ouverte | Self::Prise | Self::EnAttente)
    }
}

impl PrioriteAfaire {
    pub fn code(self) -> &'static str {
        match self {
            Self::Bloquante => "bloquante",
            Self::Due => "due",
            Self::AFaire => "a_faire",
            Self::Information => "information",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "bloquante" => Self::Bloquante,
            "due" => Self::Due,
            "a_faire" => Self::AFaire,
            "information" => Self::Information,
            _ => return None,
        })
    }
    /// Rang de tri : bloquantes d'abord (§8.5).
    pub fn rang(self) -> u8 {
        match self {
            Self::Bloquante => 0,
            Self::Due => 1,
            Self::AFaire => 2,
            Self::Information => 3,
        }
    }
}

/// Ce que la carte propose : une action en deux gestes (aperçu puis confirmation) ou un lien
/// profond vers l'écran du propriétaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionTache {
    /// `POST /afaire/{id}/apercu` puis `POST /afaire/{id}/executer` (jamais à l'ouverture d'une adresse).
    Carte {
        libelle: String,
    },
    Lien {
        libelle: String,
        lien: String,
    },
}

/// Une tâche telle qu'une personne la voit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacheAfaire {
    pub id: Uuid,
    pub nature: String,
    /// Titre neutre composé par le propriétaire (R-PO5).
    pub titre: String,
    pub eleve_id: Option<Uuid>,
    /// Prénom de l'enfant concerné (regroupement par enfant, ordre alphabétique stable, §8.5).
    pub eleve_prenom: Option<String>,
    pub priorite: PrioriteAfaire,
    pub etat: EtatAfaire,
    pub echeance_le: Option<DateTime<Utc>>,
    /// « Nouveau » : booléen personnel, pas une preuve de lecture (§29.4).
    pub nouveau: bool,
    pub action: Option<ActionTache>,
    /// Année d'origine affichée tant que la tâche est ouverte après la bascule (§8.9).
    pub annee: Option<String>,
}

/// Regroupement d'affichage (par enfant, §8.6) : calcul, jamais stocké.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeAfaire {
    pub eleve_id: Option<Uuid>,
    pub eleve_prenom: Option<String>,
    pub taches: Vec<TacheAfaire>,
}

/// `GET /afaire` : l'à-faire d'une personne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AFaire {
    /// Tâches actionnables maintenant, triées (bloquantes, échéance, ouverture).
    pub taches: Vec<TacheAfaire>,
    /// Même liste regroupée par enfant (familles).
    pub groupes: Vec<GroupeAfaire>,
    /// Nombre de tâches actionnables et **visibles** (pastille, R-PO4 ; gelé pendant la plage calme, R-PO31).
    pub compteur: i64,
    /// Mention sobre quand un lien ne porte pas l'autorité parentale (R-PO8).
    pub mention: Option<String>,
}

/// Aperçu d'une action avant exécution (§8.7 : jamais d'action sans aperçu pour une décision).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuAction {
    pub tache_id: Uuid,
    pub texte: String,
    /// Champs à fournir (aucun : l'action est confirmée telle quelle).
    pub champs: Vec<ChampAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampAction {
    pub code: String,
    pub libelle: String,
    pub obligatoire: bool,
    /// Valeurs permises (liste fermée) ou vide pour un texte court.
    pub choix: Vec<String>,
}

/// Corps de `POST /afaire/{id}/executer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAction {
    /// Confirmation explicite après aperçu (jamais implicite).
    pub confirme: bool,
    #[serde(default)]
    pub motif: Option<String>,
    /// Réponse choisie parmi les valeurs de la carte (`accepte`, `refuse` : autorisation, consentement).
    #[serde(default)]
    pub decision: Option<String>,
}

/// Réponse d'exécution. Pour une tâche déjà traitée (par qui que ce soit) : même corps, même code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatAction {
    pub fait: bool,
    pub message: String,
}

/// Message unique pour toute démarche déjà traitée (R-PO7, R-PO38).
pub const MESSAGE_DEJA_TRAITEE: &str = "Cette démarche est déjà traitée.";
/// Mention affichée à un responsable sans exercice de l'autorité parentale (R-PO8).
pub const MENTION_SANS_AUTORITE: &str =
    "Les décisions sont prises par le ou les titulaires de l'autorité parentale.";

/// Rapport d'un rapprochement (§8.10) : des comptes, **jamais de nom** (E14).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportRapprochement {
    pub natures: Vec<LigneRapprochement>,
    pub ajoutees: i64,
    pub fermees: i64,
    pub destinataires_ajoutes: i64,
    pub destinataires_retires: i64,
    pub rouvertes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneRapprochement {
    pub nature: String,
    pub source: i64,
    pub index: i64,
    pub ajoutees: i64,
    pub fermees: i64,
}
