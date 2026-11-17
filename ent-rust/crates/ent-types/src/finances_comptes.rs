//! Socle 24, lots F1 et F2 : journal financier, vue guichet, comptes débiteurs, payeurs désignés,
//! conséquences d'un changement de payeur, prises en charge d'organismes, tâches. Routes
//! `ent-api/src/finances_comptes.rs` ; l'autorité est `ent-db` (`facturation_*.rs`). Aucune logique
//! métier ici.
//!
//! Montants : **centimes entiers** (`i64`). Parts : **points de base** (`i32`, 10 000 = 100 %), jamais
//! de flottant. Les identifiants d'IBAN n'apparaissent dans aucun de ces types (le 24 ne voit jamais
//! l'IBAN, R-F22).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// F1 : journal et vue guichet
// ---------------------------------------------------------------------------

/// Une ligne du journal financier (ajout seul, R-F27).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLigneDto {
    pub id: Uuid,
    pub le: DateTime<Utc>,
    pub acteur_user_id: Option<Uuid>,
    pub casquette: String,
    pub chemin: String,
    pub action: String,
    pub objet_type: String,
    pub objet_id: Option<Uuid>,
    pub compte_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub montant_centimes: Option<i64>,
    pub motif: Option<String>,
    pub detail: Option<String>,
}

/// Comment le payeur est identifié au guichet (CF-95) : sans l'une de ces trois preuves, aucune
/// information n'est donnée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthentificationGuichet {
    /// Le payeur est devant le guichet (pièce d'identité vue).
    Presentiel,
    /// Le payeur a ouvert sa session dans l'espace famille.
    EspaceConnecte,
    /// Rappel au numéro enregistré au dossier.
    RappelNumeroEnregistre,
}

/// Quota quotidien d'actions au guichet par agent (E3) : 1 à 1 000, 30 par défaut.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuotaGuichetBody {
    pub par_jour: i32,
}

/// Résultat minimal d'une recherche au guichet (aucune adresse, aucun solde).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuichetPayeurDto {
    pub compte_id: Uuid,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuichetEcheanceDto {
    pub piece_numero: String,
    pub date_echeance: NaiveDate,
    pub montant_centimes: i64,
    pub regle_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuichetDocumentDto {
    pub facture_id: Uuid,
    pub numero: String,
    pub type_piece: String,
    pub statut: String,
    pub total_centimes: i64,
}

/// La vue guichet : le solde, les échéances et les documents du seul payeur présent (R-F37).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuichetCompteDto {
    pub compte_id: Uuid,
    pub libelle: String,
    /// Total des pièces émises moins règlements et avoirs : positif = dû, négatif = crédit.
    pub solde_centimes: i64,
    pub echeances: Vec<GuichetEcheanceDto>,
    /// Absent pour l'accueil (vue sans documents).
    pub documents: Option<Vec<GuichetDocumentDto>>,
}

// ---------------------------------------------------------------------------
// F2 : comptes débiteurs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    /// `personne`, `organisme` ou `foyer` (foyer historique à vérifier).
    pub type_debiteur: String,
    pub personne_id: Option<Uuid>,
    pub libelle_facturation: String,
    pub siret: Option<String>,
    pub reference_client: Option<String>,
    pub organisme_public: bool,
    pub chorus_pro: bool,
    pub etat: String,
    pub a_verifier: bool,
    pub a_verifier_motif: Option<String>,
    pub entite_facturation_id: Uuid,
    pub origine: String,
}

/// Mon compte, vu par la famille ou l'élève majeur (F3) : rien des champs de gestion (SIRET, état de reprise,
/// origine, entité de facturation, identifiant de personne).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonCompteDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub type_debiteur: String,
    pub libelle_facturation: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComptesFiltre {
    pub a_verifier: Option<bool>,
    pub type_debiteur: Option<String>,
}

/// Création d'un compte d'organisme payeur (FAC-256) : employeur, comité d'entreprise, aide sociale à
/// l'enfance, mairie. Aucune donnée scolaire n'y figure.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreerCompteOrganismeBody {
    pub libelle: String,
    pub siret: Option<String>,
    pub reference_client: Option<String>,
    #[serde(default)]
    pub organisme_public: bool,
    #[serde(default)]
    pub chorus_pro: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportRepriseDto {
    pub comptes_crees: i32,
    pub pieces_rattachees: i32,
    pub foyers_a_verifier: i32,
    pub designations_creees: i32,
    /// Élèves dont les parts de payeurs n'étaient pas valides (total ni 0 ni 100) : non repris, à vérifier (M9).
    pub eleves_a_verifier: i32,
}

// ---------------------------------------------------------------------------
// F2 : payeurs désignés
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPayeurDto {
    pub compte_id: Uuid,
    pub libelle: String,
    pub part_bps: i32,
    pub rang: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayeursDto {
    pub eleve_id: Uuid,
    pub categorie: String,
    pub date: NaiveDate,
    pub parts: Vec<PartPayeurDto>,
    /// `designation`, `liens` (adaptateur du socle 4) ou `aucune`.
    pub origine: String,
    /// Faux : la catégorie n'est pas facturée (payeur manquant).
    pub complet: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartBody {
    pub compte_id: Uuid,
    pub part_bps: i32,
    /// 1 (défaut) = payeur principal ; 2 = payeur suivant.
    pub rang: Option<i16>,
}

/// Pièce de répartition (FAC-217) : jugement, convention homologuée, accord des deux.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceBody {
    pub nature: String,
    pub reference: String,
    pub document_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignerPayeursBody {
    /// `toutes` ou une catégorie précise.
    pub categorie: String,
    pub parts: Vec<PartBody>,
    pub date_effet: NaiveDate,
    pub piece: Option<PieceBody>,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatDesignationDto {
    pub groupe_id: Uuid,
    /// `effective` ou `en_attente` (acceptation du payeur désigné attendue, R-F30).
    pub statut: String,
    /// Conséquences proposées sur les pièces déjà émises (aucune n'est touchée).
    pub consequences_proposees: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoindrePieceBody {
    pub piece: PieceBody,
}

/// Transfert du payeur des parents à l'élève majeur (FAC-253, CF-84).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayeurMajeurBody {
    pub date_effet: NaiveDate,
    /// Catégories qui passent à l'élève ; vide = `toutes`.
    #[serde(default)]
    pub categories: Vec<String>,
    pub motif: Option<String>,
}

/// Demande d'un parent seul (CF-75) : crée une tâche, ne change rien.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemandeRepartitionBody {
    pub eleve_id: Uuid,
    pub categorie: String,
    pub commentaire: Option<String>,
}

/// Accusé de réception d'une demande de répartition (E1) : l'identifiant de **sa propre** demande et son
/// statut ; jamais la tâche du secrétariat ni le commentaire d'un autre payeur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccuseDemandeDto {
    pub id: Uuid,
    pub statut: String,
}

// ---------------------------------------------------------------------------
// F2 : conséquences d'un changement de payeur
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsequenceDto {
    pub id: Uuid,
    pub designation_groupe_id: Uuid,
    pub eleve_id: Uuid,
    pub categorie: String,
    pub date_effet: NaiveDate,
    /// `credit` : avoir proposé au payeur sortant ; `debit` : ce qui revient au payeur entrant.
    pub sens: String,
    pub compte_id: Uuid,
    pub ligne_id: Uuid,
    pub facture_id: Uuid,
    pub mois_total: i32,
    pub mois_apres: i32,
    pub montant_centimes: i64,
    pub statut: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionConsequencesBody {
    pub appliquer: bool,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatConsequencesDto {
    pub appliquees: i32,
    pub refusees: i32,
    /// Brouillons d'avoir créés (à émettre : geste humain N3).
    pub avoirs_brouillons: Vec<Uuid>,
    /// Pièces brouillon au payeur entrant (à émettre).
    pub factures_brouillons: Vec<Uuid>,
}

// ---------------------------------------------------------------------------
// F2 : prises en charge d'organismes
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriseEnChargeBody {
    pub compte_id: Uuid,
    pub eleve_id: Uuid,
    pub reference: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub categories: Vec<String>,
    /// Plafond en centimes ; absent = sans plafond.
    pub plafond_centimes: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriseEnChargeDto {
    pub id: Uuid,
    pub compte_id: Uuid,
    pub eleve_id: Uuid,
    pub reference: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub categories: Vec<String>,
    pub plafond_centimes: Option<i64>,
    pub statut: String,
}

/// Réactivation d'une prise en charge échue (M1) : nouvelle date de fin et motif obligatoires, journalisés.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactiverPriseBody {
    pub au: NaiveDate,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportAlertesDto {
    pub a_renouveler: i32,
    pub echues: i32,
}

// ---------------------------------------------------------------------------
// F2 : tâches
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacheDto {
    pub id: Uuid,
    pub nature: String,
    pub eleve_id: Option<Uuid>,
    pub compte_id: Option<Uuid>,
    pub categorie: Option<String>,
    pub depuis: NaiveDate,
    pub detail: Option<String>,
    pub statut: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TachesFiltre {
    /// `ouverte` (défaut), `resolue`, `abandonnee`.
    pub statut: Option<String>,
}
