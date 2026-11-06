//! Évaluations, conseils de classe, bulletins et livrets (socle 2, lot S2-7).
//!
//! Types échangés par les routes `/evaluations/*` d'`ent-api` (produits tels quels par
//! `ent-db`). Les moyennes sont **déjà arrondies** par le serveur selon la règle de
//! l'établissement (`arrondi`) : le frontend n'en recalcule aucune.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────────── Référentiels ─────────────────────────────

/// Un service d'enseignement (prof × matière × classe ou groupe) évaluable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceEvaluationDto {
    pub service_id: Uuid,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    /// Code de la classe ou du groupe (« 4B », « 4B-G1 »).
    pub cible_code: String,
    pub matiere_id: Uuid,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub professeur_principal: bool,
}

/// Période de notes et son verrou (`ouverte`, `partielle_appreciations`, `fermee`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriodeNotesDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub statut: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatutPeriodeBody {
    pub statut: String,
}

/// Règles de calcul de l'établissement (`arrondi` : `centieme`, `dixieme`, `demi_point`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglesCalculDto {
    pub arrondi: String,
    pub absent_compte_zero: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoefMatiereBody {
    pub classe_id: Uuid,
    pub matiere_id: Uuid,
    pub coef: f64,
}

// ───────────────────────────── Grille ─────────────────────────────

/// Moyenne, minimum, maximum d'une série (arrondis) ; `effectif` = élèves comptés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StatsDto {
    pub moyenne: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub effectif: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevoirGrilleDto {
    pub id: Uuid,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
    pub publie_le: Option<DateTime<Utc>>,
    pub stats: StatsDto,
}

/// Résultat d'un élève à un devoir : `statut` = `note`, `absent`, `dispense`, `non_note`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultatDto {
    pub devoir_id: Uuid,
    pub statut: String,
    pub valeur: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LigneGrilleDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub resultats: Vec<ResultatDto>,
    pub moyenne: Option<f64>,
    pub appreciation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrilleDto {
    pub service: ServiceEvaluationDto,
    pub periode: PeriodeNotesDto,
    /// Saisie et modification des notes possibles (période `ouverte`).
    pub saisie_ouverte: bool,
    /// Saisie des appréciations possible (`ouverte` ou `partielle_appreciations`).
    pub appreciations_ouvertes: bool,
    pub arrondi: String,
    pub devoirs: Vec<DevoirGrilleDto>,
    pub lignes: Vec<LigneGrilleDto>,
    /// Statistiques des moyennes de la matière sur la grille.
    pub stats: StatsDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreerDevoirGrilleBody {
    pub service_id: Uuid,
    pub periode_id: Uuid,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaisieResultatBody {
    pub eleve_id: Uuid,
    pub statut: String,
    pub valeur: Option<f64>,
}

/// Saisie groupée sur un devoir : seulement les cellules modifiées.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaisieGrilleBody {
    pub resultats: Vec<SaisieResultatBody>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusSaisieDto {
    pub eleve_id: Uuid,
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RapportSaisieDto {
    pub enregistres: u32,
    pub inchanges: u32,
    pub refuses: Vec<RefusSaisieDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrigerNoteBody {
    pub eleve_id: Uuid,
    pub statut: String,
    pub valeur: Option<f64>,
    pub motif: String,
}

// ───────────────────────────── Vue classe ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatiereColonneDto {
    pub matiere_id: Uuid,
    pub libelle: String,
    pub coef: f64,
    pub stats: StatsDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LigneClasseDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    /// Même ordre que `VueClasseDto::matieres`.
    pub moyennes: Vec<Option<f64>>,
    pub moyenne_generale: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VueClasseDto {
    pub classe_id: Uuid,
    pub classe_code: String,
    pub periode: PeriodeNotesDto,
    pub arrondi: String,
    pub matieres: Vec<MatiereColonneDto>,
    pub lignes: Vec<LigneClasseDto>,
    pub stats_generale: StatsDto,
}

// ───────────────────────────── Appréciations ─────────────────────────────

/// Appréciation de matière (`matiere_id`) ou générale du professeur principal (`None`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppreciationBody {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub texte: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppreciationDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub texte: String,
    /// Alerte EVA-06 : matières avec moyenne mais sans appréciation (générale seulement).
    pub matieres_sans_appreciation: Vec<String>,
}

// ───────────────────────────── Conseil de classe ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreerConseilBody {
    pub classe_id: Uuid,
    pub periode_id: Uuid,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub lieu: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConseilDto {
    pub id: Uuid,
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub periode_id: Option<Uuid>,
    pub periode_libelle: String,
    pub jour: NaiveDate,
    pub heure: NaiveTime,
    pub lieu: String,
    pub statut: String,
}

/// Avis du conseil sur un vœu (ORI-02) et réponse de la famille (ORI-03).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoeuAvisDto {
    pub voeu_id: Uuid,
    /// Avis du conseil (masqué aux familles avant la publication du bulletin).
    pub avis_id: Option<Uuid>,
    pub rang: i16,
    pub filiere_libelle: String,
    pub avis: Option<String>,
    pub motivation: String,
    pub reponse_famille: Option<String>,
}

/// Décision du conseil pour un élève (EVA-09). Mentions et décisions [H].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionConseilDto {
    pub appreciation: String,
    pub mention: Option<String>,
    pub decision_code: Option<String>,
    pub avis_orientation: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparationEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub moyenne_generale: Option<f64>,
    /// Matières avec moyenne mais sans appréciation du professeur.
    pub appreciations_manquantes: Vec<String>,
    pub appreciation_generale: Option<String>,
    pub absences: i64,
    pub absences_non_justifiees: i64,
    pub retards: i64,
    pub sanctions: i64,
    pub decision: Option<DecisionConseilDto>,
    pub voeux: Vec<VoeuAvisDto>,
}

/// Tableau de préparation du conseil (EVA-08), calculé à la demande.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparationConseilDto {
    pub conseil: ConseilDto,
    pub periode: Option<PeriodeNotesDto>,
    pub arrondi: String,
    pub moyenne_classe: Option<f64>,
    pub appreciations_manquantes: u32,
    pub appreciations_generales_manquantes: u32,
    /// L'acteur peut saisir les décisions (direction ou professeur principal).
    pub peut_decider: bool,
    pub eleves: Vec<PreparationEleveDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionConseilBody {
    pub eleve_id: Uuid,
    pub appreciation: String,
    pub mention: Option<String>,
    pub decision_code: Option<String>,
    pub avis_orientation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvisVoeuBody {
    pub voeu_id: Uuid,
    pub avis: String,
    pub motivation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReponseFamilleBody {
    pub reponse: String,
}

// ───────────────────────────── Bilan de cycle ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComposanteNiveauDto {
    pub composante: String,
    pub niveau: i16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BilanCycleBody {
    pub eleve_id: Uuid,
    pub cycle: i16,
    pub composantes: Vec<ComposanteNiveauDto>,
    pub synthese: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BilanCycleDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub cycle: i16,
    pub statut: String,
    pub synthese: String,
    pub composantes: Vec<ComposanteNiveauDto>,
    pub valide_le: Option<DateTime<Utc>>,
}

// ───────────────────────────── Bulletins ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulletinLigneDto {
    pub matiere: String,
    pub coef: f64,
    pub moyenne: Option<f64>,
    pub moyenne_classe: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub appreciation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulletinConseilDto {
    pub appreciation: String,
    pub mention: Option<String>,
    pub decision_code: Option<String>,
    pub avis_orientation: Option<String>,
}

/// Instantané figé d'un bulletin : tout ce que le PDF affiche, et rien d'autre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulletinContenuDto {
    pub etablissement: String,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub classe: String,
    pub periode_code: String,
    pub periode_libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub arrondi: String,
    pub lignes: Vec<BulletinLigneDto>,
    pub moyenne_generale: Option<f64>,
    pub moyenne_generale_classe: Option<f64>,
    pub appreciation_generale: Option<String>,
    pub conseil: Option<BulletinConseilDto>,
    pub absences: i64,
    pub absences_non_justifiees: i64,
    pub retards: i64,
    pub version: i32,
    pub motif: Option<String>,
}

/// Une version de bulletin (`genere`, `publie`, `remplace`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulletinDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub periode_id: Uuid,
    pub version: i32,
    pub statut: String,
    pub empreinte: String,
    pub genere_le: DateTime<Utc>,
    pub publie_le: Option<DateTime<Utc>>,
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulletinDetailDto {
    pub bulletin: BulletinDto,
    pub contenu: BulletinContenuDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenererBulletinsBody {
    pub classe_id: Uuid,
    pub periode_id: Uuid,
    /// Obligatoire quand une version existe déjà (régénération après correction).
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublierBulletinsBody {
    pub classe_id: Uuid,
    pub periode_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RapportBulletinsDto {
    /// Versions créées ou publiées par l'opération.
    pub bulletins: Vec<BulletinDto>,
    /// Élèves dont le bulletin était déjà à jour (aucune nouvelle version).
    pub inchanges: u32,
}

// ───────────────────────────── Consultation famille / élève ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleveEvaluationDto {
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
    pub statut: String,
    pub valeur: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleveMatiereDto {
    pub matiere_id: Uuid,
    pub matiere_libelle: String,
    pub moyenne: Option<f64>,
    pub moyenne_classe: Option<f64>,
    pub evaluations: Vec<ReleveEvaluationDto>,
}

/// Relevé d'un élève pour une période : **seulement les évaluations publiées**.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleveEleveDto {
    pub eleve_id: Uuid,
    pub periode: PeriodeNotesDto,
    pub arrondi: String,
    pub matieres: Vec<ReleveMatiereDto>,
    pub moyenne_generale: Option<f64>,
}

// ───────────────────────────── Exports ─────────────────────────────

/// Fichier de livret (LSU, LSL) et avertissement [NV] à afficher avant tout usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportLivretDto {
    pub id: Uuid,
    pub format: String,
    pub nom_fichier: String,
    pub contenu: String,
    pub avertissement: String,
    pub anomalies: Vec<String>,
    pub nb_eleves: u32,
    pub empreinte: String,
}
