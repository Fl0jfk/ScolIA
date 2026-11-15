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
    /// Lot P4 : prises échues relâchées (POR-156).
    #[serde(default)]
    pub prises_liberees: i64,
    /// Lot P4 : tâches orphelines rendues à leur file de repli (R-PO9).
    #[serde(default)]
    pub reaffectees_repli: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneRapprochement {
    pub nature: String,
    pub source: i64,
    pub index: i64,
    pub ajoutees: i64,
    pub fermees: i64,
}

// ───────────────────────── Lot P4 : files des personnels (§18.3, POR-150 à POR-169) ─────────────────────────

/// Niveau d'une prise en charge dans une file à deux niveaux (POR-254, POR-255) : celui qui **prépare**
/// (instruit, rassemble) n'est pas celui qui **décide**. Une file à un niveau n'a que `Decider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NiveauPrise {
    Preparer,
    Decider,
}

impl NiveauPrise {
    pub fn code(self) -> &'static str {
        match self {
            Self::Preparer => "preparer",
            Self::Decider => "decider",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "preparer" => Some(Self::Preparer),
            "decider" => Some(Self::Decider),
            _ => None,
        }
    }
}

/// D'où vient la file d'une tuile : l'index des tâches (29) ou les files de demandes (N1). Une tuile ne
/// crée **aucune** file : elle lit celles des propriétaires (jamais un second modèle de file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigineFile {
    Afaire,
    Demandes,
}

/// Une tuile de « Mes files » : des nombres et une échéance, jamais de contenu (POR-150).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuileFile {
    pub code: String,
    pub libelle: String,
    pub origine: OrigineFile,
    /// Tâches (ou demandes) ouvertes, prises ou non.
    pub ouvertes: i64,
    /// Parmi elles, celles dont l'échéance est passée ; `None` quand la file du propriétaire ne publie
    /// pas d'échéance (demandes) : jamais un faux zéro.
    pub en_retard: Option<i64>,
    /// Prises par moi (index) : « ce que je dois traiter ».
    pub prises_par_moi: i64,
    /// Prises par un collègue (index).
    pub prises_par_autres: i64,
    /// Plus ancienne échéance (ou plus ancien dépôt pour les demandes).
    pub plus_ancienne: Option<DateTime<Utc>>,
    /// Écran du propriétaire où la file se traite (POR-151).
    pub lien: Option<String>,
}

/// `GET /afaire/files` : « Mes files ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MesFiles {
    pub tuiles: Vec<TuileFile>,
}

/// Qui a pris une tâche (POR-155) : « prénom + initiale », jamais d'adresse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriseVue {
    pub par_moi: bool,
    /// « Camille D. » — visible des seuls membres de la file.
    pub par: String,
    pub niveau: NiveauPrise,
    pub depuis: DateTime<Utc>,
    pub jusqu_a: DateTime<Utc>,
}

/// Une tâche dans la file d'un membre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacheFile {
    pub id: Uuid,
    pub nature: String,
    pub titre: String,
    pub eleve_prenom: Option<String>,
    pub priorite: PrioriteAfaire,
    pub etat: EtatAfaire,
    pub echeance_le: Option<DateTime<Utc>>,
    /// Prise « préparer » (s'il y en a une) et prise « décider » : au plus une de chaque.
    pub prises: Vec<PriseVue>,
    /// « Préparée par X » (file à deux niveaux, POR-254) quand la tâche attend sa décision.
    pub prepare_par: Option<String>,
    pub action: Option<ActionTache>,
}

/// Un collègue vers qui réattribuer (membre de la même file) : « prénom + initiale ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collegue {
    pub compte_id: Uuid,
    pub nom: String,
}

/// `GET /afaire/files/{code}` : le contenu d'une file pour l'un de ses membres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAfaire {
    pub code: String,
    pub libelle: String,
    /// La file a deux niveaux (préparer / décider) : l'écran propose les deux.
    pub deux_niveaux: bool,
    pub taches: Vec<TacheFile>,
    /// Vrai pour un responsable de service (`afaire.file.reattribuer`) : l'écran propose la réattribution.
    pub peut_reattribuer: bool,
    /// Collègues de la file vers qui réattribuer (responsable de service seulement) : « prénom + initiale ».
    pub collegues: Vec<Collegue>,
}

/// Corps de `POST /afaire/{id}/prendre` (le niveau par défaut est « décider »).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpsPrise {
    #[serde(default)]
    pub niveau: Option<NiveauPrise>,
}

/// Corps de `POST /afaire/{id}/reattribuer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpsReattribution {
    pub vers_compte_id: Uuid,
    #[serde(default)]
    pub niveau: Option<NiveauPrise>,
}

/// Message unique quand une tâche est déjà prise par un collègue (jamais « qui » à un non-membre).
pub const MESSAGE_DEJA_PRISE: &str = "Cette tâche est déjà prise en charge par un collègue.";

/// Une ligne de supervision : des nombres par file et par nature, jamais un nom ni un classement
/// de personnes (POR-161). L'ordre est alphabétique, **jamais** par volume ni par retard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneSupervision {
    pub file: String,
    pub libelle: String,
    pub nature: String,
    pub ouvertes: i64,
    pub en_retard: i64,
    /// Délai médian de traitement (minutes) des tâches closes depuis 7 jours ; `None` si aucune (POR-162).
    pub delai_median_minutes: Option<i64>,
}

/// `GET /afaire/supervision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Supervision {
    pub lignes: Vec<LigneSupervision>,
    /// Tâches ouvertes sans destinataire actif ni file de repli (POR-168) : doit rester à zéro.
    pub orphelines: i64,
}

/// Une tâche orpheline (POR-168) : visible de la direction avec son titre neutre seulement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Orpheline {
    pub id: Uuid,
    pub nature: String,
    pub titre: String,
    pub ouverte_le: DateTime<Utc>,
}

/// Rapport de la réparation des orphelines (rapprochement) : des comptes, jamais de nom.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportFiles {
    /// Prises échues libérées (POR-156).
    pub prises_liberees: i64,
    /// Tâches rendues à leur file de repli (aucun destinataire actif, compte parti, etc.).
    pub reaffectees_repli: i64,
}
