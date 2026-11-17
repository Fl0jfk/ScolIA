//! Socle 3, lots I3 et I4 — types d'échange des pièces d'inscription et des accords des responsables.
//!
//! Aucune donnée n'est propre à un seul client : les mêmes types servent l'API du personnel, l'espace de la
//! famille (jeton de préinscription) et l'écran public « sans compte » d'un responsable. Les états restent
//! des chaînes (`"deposee"`, `"donne"`…) comme dans le reste du module.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Une condition d'une règle de pièce. Booléennes : `foyers_distincts`, `autorite_exclusive`,
/// `responsable_tuteur`, `autre_etablissement` (`valeur` = `"true"` ou `"false"`) ; textuelles : `niveau`,
/// `regime`, `nature` (plusieurs lignes de même clé = « l'une ou l'autre »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionPieceDto {
    pub cle: String,
    pub valeur: String,
}

/// Une règle de pièce d'une campagne : quelle pièce, quand, bloquante ou non, sous quelle condition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglePieceDto {
    /// Code du catalogue du socle 7 (`livret_famille`…) ou code libre propre à la campagne (pièce « autre »).
    pub code_type: String,
    pub libelle: String,
    #[serde(default)]
    pub consigne: Option<String>,
    /// `depot`, `admission`, `confirmation` ou `rentree`.
    pub moment: String,
    /// Empêche la validation tant qu'elle n'est pas acceptée ou dispensée.
    pub bloquante: bool,
    /// `dossier` ou `demarche` (pièce de famille : déposée une fois pour tous les enfants).
    pub portee: String,
    #[serde(default)]
    pub conditions: Vec<ConditionPieceDto>,
}

/// `PUT /inscriptions/campagnes/{id}/pieces-regles` : remplace l'ensemble des règles.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglesPiecesBody {
    pub regles: Vec<ReglePieceDto>,
}

/// Une pièce attendue d'un dossier, avec l'état de son fichier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceAttendueDto {
    pub code: String,
    pub libelle: String,
    pub consigne: Option<String>,
    pub moment: String,
    pub bloquante: bool,
    pub portee: String,
    /// `attendue`, `deposee`, `acceptee`, `refusee`, `dispensee`, `reprise`.
    pub statut: String,
    /// État du fichier au socle 7 : `en_quarantaine`, `recue`, `acceptee`, `refusee`, `infectee`, `rejetee`.
    pub fichier_etat: Option<String>,
    pub nb_pages: Option<i32>,
    pub motif_refus: Option<String>,
    /// Une pièce valable déjà fournie dans la démarche (autre enfant) : à proposer, jamais utilisée en silence.
    pub proposition_dossier_id: Option<Uuid>,
    pub dispense_motif: Option<String>,
    /// Conseil de dépôt du catalogue.
    pub conseil: String,
}

/// `POST …/pieces/{code}/fichiers` : un PDF ou une à cinquante photos (JPEG, PNG), en base64, dans l'ordre
/// des pages. Le type réel est contrôlé par le serveur sur le contenu, jamais d'après le nom ou l'en-tête.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotFichiersBody {
    pub fichiers: Vec<String>,
}

/// Résultat d'un dépôt : la pièce attend l'analyse (antivirus, conversion en PDF) puis le contrôle humain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotPieceDto {
    pub code: String,
    pub statut: String,
    pub fichier_etat: String,
}

/// `POST …/pieces/{code}/reutiliser` : reprendre la pièce déjà fournie pour un autre enfant de la démarche.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReutilisationPieceBody {
    pub source_dossier_id: Uuid,
}

/// `POST /inscriptions/dossiers/{id}/pieces/{code}/controle`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlePieceBody {
    /// `accepter`, `refuser` ou `dispenser`.
    pub decision: String,
    /// Obligatoire pour refuser (motif type : `illisible`, `incomplet`, `perime`, `ne_correspond_pas`,
    /// `mauvais_document`, `autre`) et pour dispenser (motif libre, 3 à 500 caractères).
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub commentaire: Option<String>,
    /// Date lue sur la pièce (types à validité variable).
    #[serde(default)]
    pub valable_jusqu_au: Option<NaiveDate>,
}

/// Bilan de la purge des pièces des dossiers non aboutis.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilanPurgeInscriptionDto {
    pub dossiers: usize,
    pub fichiers_detruits: usize,
    pub echecs: usize,
}

// ───────────────────────────── Accords des responsables (I4) ─────────────────────────────

/// L'accord attendu (ou donné, refusé, dispensé) d'un responsable pour un dossier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccordDto {
    pub id: Uuid,
    pub declaration_id: Uuid,
    pub rang: i16,
    /// Nom de la déclaration : réservé au personnel (jamais rendu sur l'écran public).
    pub nom: String,
    pub objet: String,
    /// `attendu`, `donne`, `refuse`, `dispense`.
    pub reponse: String,
    /// `en_ligne_code`, `en_ligne_compte`, `papier`, `depot_conjoint`, `decision_direction`.
    pub moyen: Option<String>,
    pub repondu_le: Option<DateTime<Utc>>,
    /// Une demande est en cours (lien envoyé, non expiré).
    pub demande_en_cours: bool,
    pub a_piece: bool,
}

/// État des accords d'un dossier pour son objet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccordsDossierDto {
    pub objet: String,
    /// Règle de la campagne : `un_seul` ou `tous_les_titulaires`.
    pub regle: String,
    pub accords: Vec<AccordDto>,
    /// La règle est satisfaite : la décision, la confirmation et la validation sont possibles.
    pub complet: bool,
    pub bloque_desaccord: bool,
}

/// `POST /inscriptions/dossiers/{id}/accords/demander`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemanderAccordsBody {
    /// Vide : tous les titulaires qui n'ont pas répondu.
    #[serde(default)]
    pub rangs: Vec<i16>,
}

/// `POST /inscriptions/dossiers/{id}/accords/papier` : accord (ou refus) reçu sur papier, saisi par le secrétariat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccordPapierBody {
    pub rang: i16,
    /// `donne` ou `refuse`.
    pub reponse: String,
    /// Pièce `accord_responsable` du dossier, acceptée (obligatoire pour un accord).
    #[serde(default)]
    pub piece_code: Option<String>,
}

/// `POST /inscriptions/dossiers/{id}/accords/desaccord` : un titulaire a fait connaître son désaccord
/// (courrier, appel, visite) ; le dossier est bloqué.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesaccordBody {
    pub rang: i16,
    pub motif: String,
}

/// `POST /inscriptions/dossiers/{id}/accords/dispenser` : accord dispensé par décision motivée de la direction
/// (danger, ordonnance de protection, décision de justice).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispenseAccordBody {
    pub rang: i16,
    pub motif: String,
    /// Pièce du dossier, acceptée (décision de justice, ordonnance) : attendue, facultative.
    #[serde(default)]
    pub piece_code: Option<String>,
}

/// `POST /inscriptions/dossiers/{id}/accords/lever` : levée d'un désaccord, avec pièce, par la direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeverDesaccordBody {
    pub rang: i16,
    /// `accord_ecrit` (le responsable revient sur son refus : pièce `accord_responsable`) ou
    /// `decision_justice` (pièce `decision_justice` : l'accord de ce responsable est dispensé).
    pub issue: String,
    pub piece_code: String,
    pub motif: String,
}

/// Ce que voit un responsable sur l'écran public : le strict nécessaire, sans donnée d'un autre responsable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccordPublicDto {
    pub etablissement: String,
    /// Prénom de l'enfant seulement.
    pub enfant_prenom: String,
    pub objet: String,
    /// `envoyee`, `repondue`, `expiree`, `annulee`.
    pub etat: String,
    pub reponse: Option<String>,
    pub expire_le: DateTime<Utc>,
}

/// `POST /public/accords/{id}/repondre`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseAccordBody {
    pub jeton: String,
    /// Code reçu par e-mail (6 chiffres).
    pub code: String,
    /// `donne` ou `refuse`.
    pub reponse: String,
}

/// `POST /public/accords/{id}/code` et `GET /public/accords/{id}` : le jeton du lien.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JetonAccordBody {
    pub jeton: String,
}

/// Réponse neutre à un envoi de code : jamais d'indication sur l'existence ou l'état d'un accord.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeEnvoyeDto {
    pub message: String,
}

/// Message rendu à la suite d'une réponse : identique pour un accord et pour un désaccord (aucune fuite).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseEnregistreeDto {
    pub message: String,
}
