//! Évaluations, lot V4 « absences, dispenses, aménagements, rattrapages » (socle 21) : types JSON des routes
//! `/eva/v4/*`. Compile pour `wasm32` (aucune dépendance serveur).
//!
//! Principes (modèle de menace du lot) :
//! * **une note non publiée n'existe pas pour la famille ni l'élève** : aucun de ces types n'est servi à une famille
//!   ni à un élève (ces routes leur répondent 403) ;
//! * **l'absence ne vaut jamais 0** : une proposition d'absent est un *statut* (`absent`), le 0 n'existe que pour une
//!   convocation de remplacement, après une absence non justifiée, par un acte explicite de la direction ;
//! * **aucune donnée de santé** : une dispense est *datée et sans motif* ; un aménagement est un *code* du catalogue
//!   fermé du socle 2 avec son libellé fonctionnel (« Tiers-temps »), jamais le plan (PAP, PPS) ni un diagnostic.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Référence réglementaire affichée avant toute attribution d'un zéro (R-01, mémento eduscol de décembre 2025).
pub const REFERENCE_ZERO: &str = "Mémento eduscol (décembre 2025) : « la note zéro ne peut pas être utilisée pour \
sanctionner l'absence non justifiée à une évaluation ordinaire » ; seule l'absence non justifiée à l'évaluation de \
remplacement permet l'attribution de la note zéro.";

/// Seuil d'alerte d'absences répétées aux évaluations dans une période (CE-27, S-114).
pub const SEUIL_ABSENCES_REPETEES: usize = 3;

// ───────────────────────────── Propositions d'absents et de dispensés ─────────────────────────────

/// Un statut **proposé** à la case d'un élève (le professeur confirme, rien n'est écrit tant qu'il ne l'a pas fait).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionDto {
    pub eleve_id: Uuid,
    /// `absent` (port 12 : l'appel de la séance) ou `dispense` (port 34).
    pub statut: String,
    /// Pour `absent` : `justifiee`, `non_justifiee` ou `en_attente`, lue au socle 12 (jamais le motif).
    pub justification: Option<String>,
    /// `appel` ou `dispense` (source du résultat s'il est confirmé).
    pub source: String,
    /// Pour `dispense` : `dispense` ou `inaptitude_totale` (jamais de motif).
    pub nature_dispense: Option<String>,
    /// CE-25 : une absence justifiée appelle la proposition « programmer un rattrapage ».
    pub rattrapage_propose: bool,
}

/// Un élève dont l'appel dit « absent » alors que le professeur a saisi une note (CE-31) : la saisie prévaut ; la vie
/// scolaire est prévenue de l'incohérence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncoherenceAppelDto {
    pub eleve_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionsDto {
    pub evaluation_id: Uuid,
    /// Une séance du cours existe-t-elle à la date de l'évaluation (CE-30 : sinon, rien n'est proposé par l'appel).
    pub seance_trouvee: bool,
    pub propositions: Vec<PropositionDto>,
    /// CE-34 : inaptitude partielle (le professeur adapte et note) : information, rien n'est pré-rempli.
    pub inaptes_partiels: Vec<Uuid>,
    pub incoherences: Vec<IncoherenceAppelDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliquerPropositionsBody {
    pub cours_id: Uuid,
    pub evaluation_id: Uuid,
    /// `None` : toutes les propositions en cours ; sinon seulement ces élèves. Le serveur **recalcule** les
    /// propositions : un statut ne vient jamais du client.
    pub eleves: Option<Vec<Uuid>>,
}

/// Issue de la confirmation d'une proposition pour un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionAppliqueeDto {
    pub eleve_id: Uuid,
    /// `ecrit`, `deja_saisi` (une note ou un statut existe : jamais écrasé) ou `non_propose`.
    pub issue: String,
    pub statut: Option<String>,
}

// ───────────────────────────── Port 34 : dispenses (simulation) ─────────────────────────────

pub const NATURES_DISPENSE: [&str; 3] = ["dispense", "inaptitude_totale", "inaptitude_partielle"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarerDispenseBody {
    pub eleve_id: Uuid,
    /// `None` : tous les enseignements.
    pub matiere_id: Option<Uuid>,
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

/// Une dispense **sans motif**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispenseDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub retiree: bool,
}

// ───────────────────────────── Port 35 : aménagements d'évaluation ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmenagementCodeDto {
    pub code: String,
    /// Libellé **fonctionnel** du catalogue (« Tiers-temps », « Consignes lues à voix haute »).
    pub libelle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveAmenageDto {
    pub eleve_id: Uuid,
    pub amenagements: Vec<AmenagementCodeDto>,
    /// CE-37 : l'AESH de l'élève est indisponible ce jour-là.
    pub aesh_indisponible: bool,
    /// Alerte au professeur quand l'aménagement ne pourra pas être assuré (« lecteur » sans AESH) : propose de reporter.
    pub alerte: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmenagementsEvaluationDto {
    pub evaluation_id: Uuid,
    pub date_eval: NaiveDate,
    pub eleves: Vec<EleveAmenageDto>,
}

// ───────────────────────────── Rattrapages et évaluation de remplacement ─────────────────────────────

pub const MODALITES_RATTRAPAGE: [&str; 4] = ["ecrit", "oral", "numerique", "autre"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanifierRattrapageBody {
    pub evaluation_id: Uuid,
    /// Plusieurs élèves : une séance de rattrapage collective (EVA-259), un identifiant de séance.
    pub eleves: Vec<Uuid>,
    pub prevu_le: NaiveDate,
    pub heure: Option<NaiveTime>,
    pub lieu: Option<String>,
    pub modalite: Option<String>,
    /// Date limite pour passer le rattrapage (défaut : le jour prévu).
    pub echeance: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeplacerRattrapageBody {
    pub prevu_le: NaiveDate,
    pub heure: Option<NaiveTime>,
    pub lieu: Option<String>,
    pub modalite: Option<String>,
    pub echeance: Option<NaiveDate>,
    pub version_attendue: i32,
    /// V3e (F-4) : motif (3 à 500 caractères) **obligatoire** pour prolonger une échéance déjà dépassée (direction
    /// seulement, `notes.corriger`) ; la prolongation est tracée (échéance avant et après, motif, auteur).
    #[serde(default)]
    pub motif: Option<String>,
}

/// Résultat d'un rattrapage : une note (ou `nn` / `disp`), **ou** une absence au rattrapage lui-même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoterRattrapageBody {
    /// Texte de case (analyseur unique) ; ignoré si `absence` est posée.
    pub texte: Option<String>,
    /// `justifiee` ou `non_justifiee` : l'élève était absent au rattrapage.
    pub absence: Option<String>,
    pub version_attendue: i32,
    /// V3d (F-4) : motif (3 à 500 caractères) **obligatoire** pour noter après l'échéance (direction seulement).
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnulerRattrapageBody {
    pub motif: String,
    pub version_attendue: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconvoquerBody {
    pub prevu_le: NaiveDate,
    pub heure: Option<NaiveTime>,
    pub lieu: Option<String>,
    pub modalite: Option<String>,
    pub echeance: Option<NaiveDate>,
}

/// EVA-174 : attribution du zéro, seulement pour une convocation de remplacement restée « absent non justifié ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttribuerZeroBody {
    /// La direction confirme avoir lu la référence réglementaire (`REFERENCE_ZERO`).
    pub confirme: bool,
    pub version_attendue: i32,
    /// V3d (M-4) : motif (3 à 500 caractères), **obligatoire** quand c'est le professeur du cours qui attribue le zéro
    /// (période ouverte) ; facultatif pour la direction (la correction après l'arrêt porte déjà un motif fixe).
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RattrapageDto {
    pub id: Uuid,
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    /// `rattrapage` ou `remplacement`.
    pub nature: String,
    pub seance_id: Uuid,
    pub precedent_id: Option<Uuid>,
    pub prevu_le: NaiveDate,
    pub heure: Option<NaiveTime>,
    pub lieu: Option<String>,
    pub modalite: String,
    pub echeance: NaiveDate,
    /// `convoque`, `passe`, `absent_justifie`, `absent_non_justifie`, `annule`.
    pub etat: String,
    /// Convoqué, mais l'échéance est dépassée au jour de l'établissement.
    pub echu: bool,
    pub version: i32,
    pub zero_attribue: bool,
    /// Le zéro est-il attribuable (remplacement, absence non justifiée constatée, pas encore attribué) ?
    pub zero_possible: bool,
}

/// Réponse d'une attribution de zéro : la référence affichée avec l'acte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZeroAttribueDto {
    pub rattrapage: RattrapageDto,
    pub reference: String,
}

// ───────────────────────────── Alertes d'absences répétées (CE-27) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceRepeteeDto {
    pub eleve_id: Uuid,
    /// Nombre d'évaluations de la période où l'élève est absent **sans absence justifiée** au socle 12.
    pub nb_evaluations: u32,
    /// Une évaluation de remplacement est-elle déjà convoquée pour l'élève ?
    pub remplacement_convoque: bool,
}
