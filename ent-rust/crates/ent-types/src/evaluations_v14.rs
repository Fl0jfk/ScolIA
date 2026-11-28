//! Évaluations, lot V14 « cohabitation de l'an 1 » (socle 21, EVA-233 à EVA-237, CE-89). Compile pour `wasm32`.
//!
//! L'an 1 du lycée pilote, les notes et les bulletins restent dans le logiciel tiers (Charlemagne, D-032). ScolIA reçoit
//! les bulletins PDF qu'il produit, les rapproche des élèves et les **distribue seul** aux familles selon les droits des
//! liens ; il exporte l'assiduité de la période pour ce logiciel et dit aux familles où consulter les notes.
//!
//! Principes :
//! * **un bulletin non rapproché n'est jamais distribué** (CE-89) ; un rapprochement automatique ne se fait que sur un
//!   critère sans ambiguïté (INE, ou nom, prénom et date de naissance tous trois concordants et uniques) ;
//! * **les destinataires sont relus au moment de la distribution** et figés : responsable au lien actif avec le droit
//!   « voir les notes », hors interdiction de communication, et le compte de l'élève ;
//! * les indices du dépôt (INE, nom lu dans le fichier) ne sont jamais montrés à une famille.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Logiciels tiers connus (liste fermée, identique à la contrainte de la base).
pub const LOGICIELS_TIERS: [&str; 2] = ["charlemagne", "autre"];

/// Statuts d'un bulletin déposé (miroir de la base).
pub const STATUTS_BULLETIN_TIERS: [&str; 6] = [
    "a_rapprocher",
    "rapproche",
    "distribue",
    "remplace",
    "retire",
    "ecarte",
];

/// `POST /eva/v14/lots` : ouvrir un dépôt pour une période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvrirLotBody {
    pub periode_id: Uuid,
    /// `charlemagne` par défaut.
    #[serde(default)]
    pub logiciel: Option<String>,
    /// Intitulé du dépôt (« Bulletins T1, 2de A à 2de F »), 120 caractères au plus ; la période par défaut.
    #[serde(default)]
    pub libelle: Option<String>,
}

/// `POST /eva/v14/lots/{id}/fichiers` : un bulletin PDF, en base64 ; les indices sont facultatifs (sinon lus dans le nom
/// du fichier quand il porte un INE).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeposerBulletinTiersBody {
    pub nom_fichier: String,
    pub contenu_base64: String,
    #[serde(default)]
    pub ine: Option<String>,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    #[serde(default)]
    pub date_naissance: Option<NaiveDate>,
}

/// `POST /eva/v14/bulletins/{id}/rapprocher` : rapprochement manuel (après aperçu), ou correction d'un rapprochement
/// automatique avant la distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapprocherBulletinBody {
    pub eleve_id: Uuid,
}

/// `POST /eva/v14/bulletins/{id}/ecarter` et `/retirer` : 3 à 500 caractères.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifBulletinTiersBody {
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotTiersDto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub periode_libelle: String,
    pub logiciel: String,
    pub libelle: String,
    pub depose_le: DateTime<Utc>,
    pub fichiers: u32,
    pub a_rapprocher: u32,
    pub rapproches: u32,
    pub distribues: u32,
    pub ecartes: u32,
}

/// Un bulletin déposé, vu du personnel (le secrétariat et la direction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulletinTiersDto {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub periode_id: Uuid,
    pub nom_fichier: String,
    pub taille: i32,
    pub statut: String,
    pub indice_ine: Option<String>,
    pub indice_nom: Option<String>,
    pub indice_prenom: Option<String>,
    pub indice_naissance: Option<NaiveDate>,
    pub eleve_id: Option<Uuid>,
    pub eleve_nom: Option<String>,
    pub eleve_prenom: Option<String>,
    /// `ine`, `identite` ou `manuel`.
    pub rapprochement: Option<String>,
    pub motif: Option<String>,
    pub distribue_le: Option<DateTime<Utc>>,
    /// Destinataires figés à la distribution (0 avant).
    pub destinataires: u32,
}

/// Contrôle avant distribution (EVA-246) et rapport de distribution (EVA-235).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportDistributionDto {
    /// `true` : contrôle seul, rien n'a été écrit.
    pub controle: bool,
    pub lot_id: Uuid,
    /// Bulletins rapprochés, distribués (ou qui le seraient).
    pub distribues: u32,
    /// Nouvelles versions qui remplacent un bulletin déjà distribué pour la même période.
    pub remplacements: u32,
    /// Restent dans la file « à rapprocher » : jamais distribués (CE-89).
    pub a_rapprocher: u32,
    /// Responsables et élèves destinataires, au total.
    pub destinataires: u32,
    /// Élèves rapprochés sans aucun destinataire en ligne : à remettre sur papier (lot d'impression).
    pub sans_destinataire: Vec<EleveSansDestinataireDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveSansDestinataireDto {
    pub bulletin_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

/// Bulletin distribué, vu de la famille ou de l'élève : ni indice, ni fichier d'origine, ni motif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulletinTiersFamilleDto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub periode_libelle: String,
    pub distribue_le: DateTime<Utc>,
    /// Document du coffre de l'élève (socle 7) qui porte le même PDF.
    pub document_id: Uuid,
}

/// Bandeau « où sont les notes » (EVA-237), lisible de tout membre de l'établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CohabitationDto {
    pub actif: bool,
    pub logiciel_libelle: String,
    pub url: Option<String>,
    pub message: String,
}

/// `PUT /eva/v14/cohabitation` (direction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglerCohabitationBody {
    pub actif: bool,
    pub logiciel_libelle: String,
    /// `https://` seulement.
    #[serde(default)]
    pub url: Option<String>,
    pub message: String,
}

/// `GET /eva/v14/assiduite?periode_id=…&classe_id=…` : ligne de l'export (aussi rendue en CSV).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneAssiduiteTiersDto {
    pub eleve_id: Uuid,
    pub ine: Option<String>,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
    /// Demi-journées d'absence qui comptent (règle d'assiduité du jour, socle 12).
    pub demi_journees: u32,
    pub justifiees: u32,
    pub non_justifiees: u32,
    pub retards: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportAssiduiteTiersDto {
    pub export_id: Uuid,
    pub periode_id: Uuid,
    pub classe_id: Uuid,
    /// Bornes réellement comptées : début de la période, fin de la période ou aujourd'hui si elle n'est pas finie.
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// `true` : la période n'est pas terminée, le décompte s'arrête à aujourd'hui.
    pub partiel: bool,
    pub lignes: Vec<LigneAssiduiteTiersDto>,
}
