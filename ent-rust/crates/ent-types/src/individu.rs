//! Transverse « la deuxième personne fondée sur la personne physique » (migration 279,
//! `docs/socles/TRANSVERSE_INDIVIDU.md`) : routes `/dossier/individu/*`.
//!
//! Une seule définition : `ent-db` réexporte ces types (`ent_db::individu`) et les sérialise tels
//! quels ; le frontend les décode. Aucune logique métier ici, seulement les listes fermées des motifs,
//! que la base et l'écran partagent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Motifs codés d'un lien posé par la direction ou le secrétariat.
pub const MOTIFS_LIEN: [&str; 4] = ["deux_comptes", "fiche_sans_compte", "doublon_agent", "autre"];
/// Motifs codés d'une levée.
pub const MOTIFS_LEVEE: [&str; 3] = ["erreur", "homonyme", "autre"];
/// Motifs codés d'une proposition écartée.
pub const MOTIFS_ECARTEE: [&str; 3] = ["homonyme", "coordonnee_partagee", "autre"];

/// Un lien d'individu actif, vu de la direction ou du secrétariat (`GET /dossier/individu/liens`).
/// Le nom d'une fiche fermée au lecteur (conflit d'intérêts, adresse protégée) vaut « Fiche protégée ».
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LienIndividu {
    pub id: Uuid,
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub nom_a: String,
    pub nom_b: String,
    /// `declaration_direction`, `declaration_confirmee`, `proposition_acceptee`.
    pub origine: String,
    pub motif_code: String,
    pub pose_le: DateTime<Utc>,
}

/// Une déclaration reçue, en attente de confirmation par le compte cible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclarationRecue {
    pub id: Uuid,
    /// Nom affiché du compte déclarant (la personne qui déclare se fait connaître de son autre compte).
    pub declarant: String,
    pub declare_le: DateTime<Utc>,
    pub expire_le: DateTime<Utc>,
}

/// Un lien qui touche l'individu du compte, vu par lui : quand et comment, **jamais** la fiche reliée
/// (un lien posé à tort désignerait une autre personne).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonLienIndividu {
    pub origine: String,
    pub motif_code: String,
    pub pose_le: DateTime<Utc>,
}

/// Ce que le compte voit de son individu (`GET /dossier/individu/moi`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonIndividu {
    pub autres_comptes: i64,
    pub fiches_reliees: i64,
    pub declarations_recues: Vec<DeclarationRecue>,
    /// Liens actifs de l'individu, les plus récents d'abord (ajouté avec les écrans ; absent d'un serveur plus ancien).
    #[serde(default)]
    pub liens: Vec<MonLienIndividu>,
}

/// Une ressemblance entre deux fiches (une au moins est un agent). Jamais la coordonnée elle-même.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PropositionIndividu {
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub nom_a: String,
    pub nom_b: String,
    /// `email`, `telephone`.
    pub raisons: Vec<String>,
}

/// Corps de `POST /dossier/individu/declarations`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclarationIndividuBody {
    pub email: String,
}

/// Corps de `POST /dossier/individu/declarations/{id}/reponse`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReponseDeclarationBody {
    pub confirmer: bool,
}

/// Corps de `POST /dossier/individu/liens`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelierFichesBody {
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub motif_code: String,
    #[serde(default)]
    pub depuis_proposition: bool,
}

/// Corps de `POST /dossier/individu/liens/{id}/lever`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeverLienBody {
    pub motif_code: String,
}

/// Corps de `POST /dossier/individu/propositions/ecarter`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EcarterPropositionBody {
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub motif_code: String,
}
