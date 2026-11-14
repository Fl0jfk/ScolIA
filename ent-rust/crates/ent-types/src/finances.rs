//! Finances reliées (socle 2, lot S2-9 ; FIN-01 → FIN-17). Routes
//! `ent-api/src/finances.rs` (préfixes `/finances`, `/scolarite`). Aucune logique
//! métier : l'autorité est `ent-db` (`finances.rs`).
//!
//! Montants : **centimes entiers** (`i64`). États (chaînes) :
//! - pièce : `type_piece` `"facture"` / `"avoir"` ; `statut` `"brouillon"`,
//!   `"emise"`, `"soldee"` ;
//! - échéance : `"a_venir"`, `"payee"`, `"en_retard"` ;
//! - prélèvement : `"prevu"`, `"exporte"`, `"execute"`, `"annule"`, `"rejete"` ;
//! - relance : `"proposee"`, `"envoyee"`, `"annulee"` ;
//! - mode de paiement d'un foyer : `"prelevement"`, `"manuel"`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Paramétrage (FIN-01, FIN-05, FIN-07)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametresFinancesDto {
    pub mois_debut: i16,
    pub nb_mois: i16,
    pub jour_echeance: i16,
    pub jours_prenotification: i16,
    pub frais_rejet_centimes: i64,
    pub rejets_avant_manuel: i16,
    pub seuil_absence_jours: i16,
    pub crediteur_nom: Option<String>,
    /// IBAN du créancier (établissement) **masqué** en lecture.
    pub crediteur_iban: Option<String>,
    pub crediteur_bic: Option<String>,
    pub crediteur_ics: Option<String>,
    pub compte_clients: String,
    pub compte_banque: String,
    pub journal_ventes: String,
    pub journal_banque: String,
    /// Paliers de relance (rang, jours de retard, libellé, frais).
    #[serde(default)]
    pub paliers: Vec<PalierRelanceDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PalierRelanceDto {
    pub rang: i16,
    pub jours_retard: i16,
    pub libelle: String,
    pub frais_centimes: i64,
}

/// Corps de `PUT /finances/parametres` : champs absents = inchangés.
/// `crediteur_iban` absent = inchangé (jamais renvoyé en clair).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametresFinancesBody {
    pub mois_debut: Option<i16>,
    pub nb_mois: Option<i16>,
    pub jour_echeance: Option<i16>,
    pub jours_prenotification: Option<i16>,
    pub frais_rejet_centimes: Option<i64>,
    pub rejets_avant_manuel: Option<i16>,
    pub seuil_absence_jours: Option<i16>,
    pub crediteur_nom: Option<String>,
    pub crediteur_iban: Option<String>,
    pub crediteur_bic: Option<String>,
    pub crediteur_ics: Option<String>,
    pub compte_clients: Option<String>,
    pub compte_banque: Option<String>,
    pub journal_ventes: Option<String>,
    pub journal_banque: Option<String>,
    pub paliers: Option<Vec<PalierRelanceDto>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    /// `scolarite`, `restauration`, `internat`, `activite`, `frais`, `bourse`, `reduction`.
    pub categorie: String,
    pub compte_produit: String,
    pub actif: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleBody {
    pub code: String,
    pub libelle: String,
    pub categorie: String,
    pub compte_produit: Option<String>,
    pub actif: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarifDto {
    pub id: Uuid,
    pub annee_scolaire: String,
    pub article_id: Uuid,
    pub article_code: String,
    pub article_libelle: String,
    pub categorie: String,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min_centimes: Option<i64>,
    pub qf_max_centimes: Option<i64>,
    pub montant_annuel_centimes: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarifBody {
    pub annee_scolaire: String,
    pub article_id: Uuid,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min_centimes: Option<i64>,
    pub qf_max_centimes: Option<i64>,
    pub montant_annuel_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReductionDto {
    pub id: Uuid,
    pub annee_scolaire: String,
    pub code: String,
    pub libelle: String,
    /// `fratrie`, `personnel`, `autre`.
    pub nature: String,
    pub rang_min: Option<i16>,
    pub taux_bps: i32,
    pub categories: Vec<String>,
    /// Élèves bénéficiaires (réductions `personnel` / `autre`).
    #[serde(default)]
    pub eleves: Vec<Uuid>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReductionBody {
    pub annee_scolaire: String,
    pub code: String,
    pub libelle: String,
    pub nature: String,
    pub rang_min: Option<i16>,
    pub taux_bps: i32,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveReductionBody {
    pub eleve_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaremeBourseDto {
    pub id: Uuid,
    pub annee_scolaire: String,
    pub bourse_type_id: Uuid,
    pub echelon: String,
    pub montant_annuel_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaremeBourseBody {
    pub annee_scolaire: String,
    pub bourse_type_id: Uuid,
    pub echelon: String,
    pub montant_annuel_centimes: i64,
}

/// `GET/PUT /finances/echeancier-type?annee=AAAA-AAAA` : dates des échéances.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheancierTypeDto {
    pub annee_scolaire: String,
    pub dates: Vec<NaiveDate>,
}

// ---------------------------------------------------------------------------
// Génération annuelle (FIN-02) et consommations (FIN-06)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationBody {
    pub annee_scolaire: String,
    /// `true` : calcule sans rien écrire.
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportGenerationDto {
    pub annee_scolaire: String,
    pub apercu: bool,
    pub eleves: i64,
    pub lignes_creees: i64,
    pub lignes_deja_presentes: i64,
    pub factures_touchees: i64,
    pub montant_centimes: i64,
    /// Élèves non facturables (sans payeur, sans tarif…) avec le motif.
    pub anomalies: Vec<AnomalieGenerationDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalieGenerationDto {
    pub eleve_id: Uuid,
    pub eleve: String,
    pub motif: String,
}

/// `POST /finances/consommations` : une source externe (repas, activité, séjour)
/// produit au plus une ligne par foyer payeur (clé `source`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsommationBody {
    /// Clé de la source, stable et unique (ex. `repas:2026-10:<eleve>`).
    pub source: String,
    pub eleve_id: Uuid,
    pub article_id: Uuid,
    pub libelle: String,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
    pub periode_debut: Option<NaiveDate>,
    pub periode_fin: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatSourceDto {
    pub lignes_creees: i64,
    pub lignes_deja_presentes: i64,
    pub factures: Vec<Uuid>,
}

// ---------------------------------------------------------------------------
// Factures, avoirs, échéances (FIN-03 → FIN-05)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactureResumeDto {
    pub id: Uuid,
    pub numero: String,
    pub type_piece: String,
    pub origine: Option<String>,
    pub statut: String,
    pub foyer_id: Uuid,
    pub foyer: String,
    pub libelle: String,
    pub annee_scolaire: Option<String>,
    pub total_centimes: i64,
    pub regle_centimes: i64,
    pub reste_centimes: i64,
    pub facture_origine_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub emise_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneFactureDto {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub eleve: Option<String>,
    pub libelle: String,
    pub categorie: Option<String>,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
    pub total_centimes: i64,
    pub periode_debut: Option<NaiveDate>,
    pub periode_fin: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheanceDto {
    pub id: Uuid,
    pub rang: i32,
    pub libelle: String,
    pub date_echeance: NaiveDate,
    pub montant_centimes: i64,
    pub couvert_centimes: i64,
    pub etat: String,
    /// Prélèvement en cours ou exécuté sur cette échéance.
    pub prelevement: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LettrageDto {
    pub code: String,
    pub piece: String,
    pub piece_id: Uuid,
    pub montant_centimes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactureDetailDto {
    pub facture: FactureResumeDto,
    pub lignes: Vec<LigneFactureDto>,
    pub echeances: Vec<EcheanceDto>,
    pub lettrages: Vec<LettrageDto>,
    pub avoirs: Vec<Uuid>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmissionBody {
    pub factures: Vec<Uuid>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportEmissionDto {
    pub emises: Vec<FactureResumeDto>,
    /// Pièces non émises avec la raison (déjà émise, vide…).
    pub refus: Vec<RefusDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusDto {
    pub id: Uuid,
    pub motif: String,
}

/// `POST /finances/factures/{id}/avoir` : avoir total (sans `montant_centimes`)
/// ou partiel (réparti sur les lignes au prorata).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvoirBody {
    pub motif: String,
    pub montant_centimes: Option<i64>,
}

// ---------------------------------------------------------------------------
// SEPA (FIN-10 → FIN-12)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparationBody {
    pub date_execution: NaiveDate,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportPreparationDto {
    pub date_execution: Option<NaiveDate>,
    pub prelevements_crees: i64,
    pub prenotifications: i64,
    pub montant_centimes: i64,
    /// Foyers écartés (paiement manuel, sans mandat actif…).
    pub ecartes: Vec<FoyerEcarteDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoyerEcarteDto {
    pub foyer_id: Uuid,
    pub foyer: String,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrelevementDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub foyer: String,
    pub facture_id: Uuid,
    pub facture_numero: String,
    pub echeance_id: Option<Uuid>,
    pub montant_centimes: i64,
    pub date_execution: NaiveDate,
    pub statut: String,
    pub presentation: i16,
    pub end_to_end_id: String,
    pub rum: String,
    /// IBAN du débiteur **masqué** (`FR76 •••• 1234`).
    pub iban_masque: String,
    pub prenotifie: bool,
    pub motif_rejet: Option<String>,
}

/// `POST /finances/prelevements/export` : `format` `pain008` (défaut) ou `csv`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPrelevementsBody {
    pub date_execution: NaiveDate,
    pub format: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionBody {
    pub date_execution: NaiveDate,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportExecutionDto {
    pub executes: i64,
    pub montant_centimes: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejetBody {
    /// Code ISO 20022 (AM04, AC04, MD01…).
    pub code: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejetDto {
    pub id: Uuid,
    pub prelevement_id: Uuid,
    pub foyer_id: Uuid,
    pub foyer: String,
    pub facture_id: Uuid,
    pub code_motif: String,
    pub libelle_motif: String,
    pub montant_centimes: i64,
    pub frais_centimes: i64,
    pub facture_frais_id: Option<Uuid>,
    /// `representation` ou `manuel`.
    pub suite: String,
    pub traite_le: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Règlements, lettrage, compte famille (FIN-08, FIN-17)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglementBody {
    pub foyer_id: Uuid,
    pub montant_centimes: i64,
    /// `especes`, `cheque`, `virement`, `cb`, `autre`.
    pub moyen: String,
    pub reference: Option<String>,
    pub recu_le: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglementDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub montant_centimes: i64,
    pub moyen: String,
    pub reference: Option<String>,
    pub recu_le: NaiveDate,
    pub non_impute_centimes: i64,
    pub imputations: Vec<ImputationDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImputationDto {
    pub facture_id: Uuid,
    pub facture_numero: String,
    pub montant_centimes: i64,
    pub lettrage: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModePaiementBody {
    /// `prelevement` ou `manuel`.
    pub mode: String,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandatResumeDto {
    pub id: Uuid,
    pub rum: String,
    pub iban_masque: String,
    pub titulaire: String,
    pub statut: String,
    pub signe_at: DateTime<Utc>,
}

/// Compte d'un foyer payeur : vu par la comptabilité ou par le payeur lui-même.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteFoyerDto {
    pub foyer_id: Uuid,
    pub foyer: String,
    pub mode_paiement: String,
    pub solde_centimes: i64,
    pub credit_centimes: i64,
    pub factures: Vec<FactureResumeDto>,
    pub echeances: Vec<EcheanceFoyerDto>,
    pub mandats: Vec<MandatResumeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheanceFoyerDto {
    pub facture_id: Uuid,
    pub facture_numero: String,
    pub echeance: EcheanceDto,
}

// ---------------------------------------------------------------------------
// Relances (FIN-13) et export comptable (FIN-16)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelanceDto {
    pub id: Uuid,
    pub facture_id: Uuid,
    pub facture_numero: String,
    pub foyer_id: Uuid,
    pub foyer: String,
    pub palier: i16,
    pub libelle: String,
    pub montant_du_centimes: i64,
    pub jours_retard: i32,
    pub frais_centimes: i64,
    pub statut: String,
    pub proposee_le: DateTime<Utc>,
    pub decide_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportRelancesDto {
    pub proposees: Vec<RelanceDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportComptableBody {
    pub debut: NaiveDate,
    pub fin: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportComptableDto {
    pub id: Uuid,
    pub numero: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub format: String,
    pub nb_ecritures: i32,
    pub total_debit_centimes: i64,
    pub total_credit_centimes: i64,
    pub empreinte: String,
    pub created_at: DateTime<Utc>,
    /// Contenu du fichier (seulement dans la réponse de création).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contenu: Option<String>,
}

// ---------------------------------------------------------------------------
// Paiement en ligne (FIN-09, Q5 non décidée)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatPaiementEnLigneDto {
    pub actif: bool,
    pub prestataire: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentionPaiementBody {
    pub facture_id: Uuid,
    pub montant_centimes: i64,
}

// ---------------------------------------------------------------------------
// Scolarité : changement de régime et sortie (D3, déclencheurs de facturation)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegimeBody {
    /// `externe`, `demi_pension`, `interne`.
    pub regime: String,
    pub date_effet: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortieBody {
    pub date_sortie: NaiveDate,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvenementScolariteDto {
    pub eleve_id: Uuid,
    pub evenement_id: Uuid,
}

/// Espace famille : comptes des foyers dont la personne connectée est payeuse.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EspaceFamilleFinancesDto {
    pub comptes: Vec<CompteFoyerDto>,
    pub paiement_en_ligne: Option<EtatPaiementEnLigneDto>,
}
