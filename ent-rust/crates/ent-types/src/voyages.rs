//! Types JSON des routes `/voyages/*` (socle 42, lots V0a et V0b : sorties, voyages, budget).
//!
//! Le JSON est **identique** à celui qu'`ent-api` produit (mêmes noms de champs) ; un test d'`ent-api`
//! désérialise de vraies réponses dans ces types. Les états restent des chaînes (`"brouillon"`,
//! `"soumise"`…) : le code d'état est celui du §8.1 de `docs/socles/42_SORTIES_VOYAGES.md`. Aucune
//! règle ici : le serveur décide (droits, états, préconditions) ; `AccesDto` ne sert qu'à masquer
//! les boutons que le serveur refuserait.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Saisie d'une sortie (création, remplacement des champs du projet à la modification).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieSaisieDto {
    pub titre: String,
    #[serde(default)]
    pub objectifs: Option<String>,
    pub nature: String,
    pub forme: String,
    pub destination: String,
    #[serde(default)]
    pub pays: Option<String>,
    #[serde(default)]
    pub destination_libelle: Option<String>,
    #[serde(default)]
    pub programme_libelle: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub classes: Vec<Uuid>,
    #[serde(default)]
    pub groupes: Vec<Uuid>,
    #[serde(default)]
    pub effectif_mode: Option<String>,
    #[serde(default)]
    pub places: Option<i32>,
    #[serde(default)]
    pub effectif_eleves_declare: Option<i32>,
    #[serde(default)]
    pub effectif_adultes_declare: Option<i32>,
    #[serde(default)]
    pub besoin_car: bool,
    #[serde(default)]
    pub car_sur_place: bool,
    #[serde(default)]
    pub point_rendez_vous: Option<String>,
    #[serde(default)]
    pub consignes_transport: Option<String>,
    #[serde(default)]
    pub places_encadrant_a_pourvoir: i32,
}

/// Modification : la version lue et les champs du projet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieModifDto {
    pub version: i32,
    #[serde(flatten)]
    pub saisie: SortieSaisieDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerieSaisieDto {
    pub libelle: String,
    /// 1 = lundi … 7 = dimanche.
    pub jour_semaine: u32,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub modele: SortieSaisieDto,
}

/// Sortie telle que le serveur la renvoie après une écriture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub serie_id: Option<Uuid>,
    pub reference: String,
    pub titre: String,
    pub objectifs: Option<String>,
    pub nature: String,
    pub forme: String,
    pub destination: String,
    pub pays: Option<String>,
    pub destination_libelle: Option<String>,
    pub programme_libelle: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub porteur_personne_id: Uuid,
    pub etat: String,
    pub etat_retour: Option<String>,
    pub motif_decision: Option<String>,
    pub effectif_mode: String,
    pub places: Option<i32>,
    pub effectif_eleves_declare: Option<i32>,
    pub effectif_adultes_declare: Option<i32>,
    pub besoin_car: bool,
    pub car_sur_place: bool,
    pub point_rendez_vous: Option<String>,
    pub consignes_transport: Option<String>,
    pub places_encadrant_a_pourvoir: i32,
    pub contournement_devis_motif: Option<String>,
    pub liste_confirmee_le: Option<DateTime<Utc>>,
    pub version: i32,
    pub cree_le: DateTime<Utc>,
    pub modifie_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieResumeDto {
    pub id: Uuid,
    pub reference: String,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub destination: String,
    pub destination_libelle: Option<String>,
    pub nature: String,
    pub forme: String,
    pub etat: String,
    pub serie_id: Option<Uuid>,
    pub porteur_nom: Option<String>,
    pub nb_inscrits: Option<i64>,
    pub acces_detail: bool,
    pub est_porteur: bool,
}

/// Droits de l'appelant sur une sortie (affichage seulement : le serveur revérifie).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccesDto {
    pub porteur: bool,
    pub encadrant: bool,
    pub detail: bool,
    pub modifier: bool,
    pub participants: bool,
    pub encadrement: bool,
    pub valider: bool,
    pub autoriser: bool,
    pub budget_valider: bool,
    pub prestataires: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicDto {
    pub id: Uuid,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeDto {
    pub id: Uuid,
    pub ordre: i16,
    #[serde(rename = "type")]
    pub type_etape: String,
    pub debut: DateTime<Utc>,
    pub fin: Option<DateTime<Utc>>,
    pub duree_minutes: Option<i32>,
    pub lieu: Option<String>,
    pub note: Option<String>,
    pub appel_requis: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeSaisieDto {
    #[serde(rename = "type")]
    pub type_etape: String,
    pub debut: DateTime<Utc>,
    #[serde(default)]
    pub fin: Option<DateTime<Utc>>,
    #[serde(default)]
    pub duree_minutes: Option<i32>,
    #[serde(default)]
    pub lieu: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub appel_requis: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncadrantDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub etat: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointAuditDto {
    pub code: String,
    /// `bloquant`, `avertissement` ou `conseil`.
    pub gravite: String,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProchaineEtapeDto {
    /// `porteur`, `direction`, `comptabilite`, `secretariat`, `personne`.
    pub qui: String,
    pub libelle: String,
    pub commande: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieDetailDto {
    pub sortie: SortieDto,
    pub porteur_nom: String,
    pub publics: Vec<PublicDto>,
    pub etapes: Vec<EtapeDto>,
    pub encadrants: Vec<EncadrantDto>,
    pub nb_inscrits: i64,
    pub nb_desistes: i64,
    pub prochaine_etape: ProchaineEtapeDto,
    pub audit: Vec<PointAuditDto>,
    pub acces: AccesDto,
    pub commandes: Vec<CommandeDto>,
}

/// Commande d'état proposée à l'appelant dans l'état actuel (calcul serveur).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandeDto {
    pub code: String,
    pub libelle: String,
    pub motif_requis: bool,
    pub danger: bool,
    /// Étapes possibles pour une réouverture.
    pub vers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalDto {
    pub a: DateTime<Utc>,
    pub action: String,
    pub etat_avant: Option<String>,
    pub etat_apres: Option<String>,
    pub champ: Option<String>,
    pub avant: Option<String>,
    pub apres: Option<String>,
    pub motif: Option<String>,
    pub acteur_nom: Option<String>,
}

/// Paramètres d'une commande d'état.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamsCommandeDto {
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub vers: Option<String>,
    /// Version de budget lue par la comptabilité (`valider-budget`) : on valide ce qu'on a lu (V0b).
    #[serde(default)]
    pub budget_numero: Option<i32>,
    #[serde(default)]
    pub budget_rev: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatAjoutDto {
    pub ajoutes: Vec<Uuid>,
    pub deja: Vec<Uuid>,
    pub refuses: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub deja_inscrit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatsClasseDto {
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub libelle: String,
    pub eleves: Vec<CandidatDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableauDirectionDto {
    pub a_valider: i64,
    pub budget_a_valider: i64,
    pub a_autoriser: i64,
    pub en_preparation: i64,
    pub ouvertes: i64,
    pub pretes: i64,
    pub a_venir: i64,
    pub validees_de_l_annee: i64,
    pub a_completer: i64,
    pub bloquees_proches: Vec<SortieResumeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoyageFamilleDto {
    pub sortie_id: Uuid,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub destination: String,
    pub destination_libelle: Option<String>,
    pub point_rendez_vous: Option<String>,
    pub etat: String,
    pub enfant_prenom: String,
    pub etat_participant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CirculaireDto {
    pub titre: String,
    pub reference: String,
    pub corps: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatSeanceDto {
    pub sortie_id: Uuid,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub ok: bool,
    pub raison: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateEcarteeDto {
    pub jour: NaiveDate,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatSerieDto {
    pub serie_id: Uuid,
    pub sorties: Vec<SortieResumeDto>,
    pub ecartees: Vec<DateEcarteeDto>,
}

// ───────────────────────── Lot V0b : budget, prix annoncé, documents, rappels ─────────────────────────
//
// Tous les montants sont des **centimes entiers** (R-SV16). Le calcul de la part des familles est
// fait par le serveur ; ces types ne contiennent aucune règle.

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalculBudgetDto {
    pub nb_payants: i64,
    pub depenses_total: i64,
    pub depenses_familles: i64,
    pub encadrement: i64,
    pub surcout_handicap: i64,
    pub recettes_total: i64,
    pub recettes_encadrement: i64,
    pub recettes_reduisant_la_part: i64,
    pub marge_pct: i64,
    pub marge: i64,
    pub cout_reel_par_eleve: Option<i64>,
    pub part_familles: Option<i64>,
    pub total_familles: i64,
    pub deficit: i64,
    pub arrondi_pris_en_charge: i64,
    pub excedent: i64,
    pub equilibre: bool,
    pub causes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetLigneDto {
    pub id: Uuid,
    pub sens: String,
    pub poste: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub source: String,
    pub motif: Option<String>,
    pub affectation: Option<String>,
    pub rang: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetVersionDto {
    pub numero: i32,
    pub etat: String,
    /// Compteur d'écritures : à renvoyer avec chaque écriture et avec la validation.
    pub rev: i32,
    pub nb_payants: i32,
    pub marge_pct: i32,
    pub marge_motif: Option<String>,
    pub motif_revision: Option<String>,
    pub part_familles_centimes: Option<i64>,
    pub cree_le: DateTime<Utc>,
    pub valide_le: Option<DateTime<Utc>>,
    pub lignes: Vec<BudgetLigneDto>,
    pub calcul: CalculBudgetDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrixFigeDto {
    pub rang: i32,
    pub montant_centimes: i64,
    pub cause: String,
    pub motif: Option<String>,
    pub fige_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionResumeDto {
    pub numero: i32,
    pub etat: String,
    pub valide_le: Option<DateTime<Utc>>,
    pub part_familles_centimes: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetDto {
    pub sortie_id: Uuid,
    pub a_budget: bool,
    pub obligatoire: bool,
    pub marge_prereglee: i32,
    pub peut_ecrire: bool,
    pub peut_valider: bool,
    pub peut_decider_hausse: bool,
    pub brouillon: Option<BudgetVersionDto>,
    pub validee: Option<BudgetVersionDto>,
    pub historique: Vec<VersionResumeDto>,
    pub prix_fige: Vec<PrixFigeDto>,
    pub prix_courant_centimes: Option<i64>,
    pub hausse_en_attente: bool,
}

/// Ligne de budget envoyée au serveur : le montant est **un texte** au format français (« 1 350,50 »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneBudgetSaisieDto {
    pub rev: i32,
    pub sens: String,
    pub poste: String,
    pub libelle: String,
    pub montant: String,
    pub motif: Option<String>,
    pub affectation: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamsBudgetDto {
    pub rev: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nb_payants: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marge_pct: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marge_motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDto {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_: String,
    pub nom: String,
    pub taille: i32,
    pub sha256: String,
    pub emis_le: DateTime<Utc>,
    pub version_budget: Option<i32>,
    pub actuel: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RappelDto {
    pub code: String,
    pub jour: String,
    pub jours_avant: i32,
    pub message: String,
}
