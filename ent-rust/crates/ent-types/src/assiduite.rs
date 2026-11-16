//! Assiduité (socle 12, lots AS14a à AS14c) : types JSON de l'appel v2, des épisodes d'absence, des
//! justificatifs, des avis aux familles et de la vue de la famille.
//!
//! Le JSON est **identique** à celui qu'`ent-api` produit (`ent-db::assiduite_as2`, `_as3`, `_as5`) : mêmes
//! noms de champs. Types de **lecture** pour l'essentiel (les champs inconnus sont ignorés) ; les corps
//! d'écriture ne portent que ce que le serveur accepte. Les états restent des chaînes (`"present"`,
//! `"a_qualifier"`…) ; leur affichage est dans `ent-web-app`. **Aucun motif libre ni note interne** ne figure
//! dans les types destinés au professeur ou à la famille (R-AS21, R-AS22).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Appel v2 (AS2) ──────────────────────────────────────────────────────────────────────────

/// Ligne d'un appel : jamais de motif.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneAppelV2Dto {
    pub eleve_id: Uuid,
    pub etat: String,
    pub minutes_retard: Option<i32>,
    pub lieu_code: Option<String>,
    /// Pré-remplie par une situation connue (déclaration, sortie, localisation…).
    pub pre_remplie: bool,
    /// Libellé court de la situation (« prévenu », « en sortie »…), sans motif.
    pub libelle_court: Option<String>,
    pub version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub arrivee_vs_attendue: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecapAppelDto {
    pub presents: i64,
    pub absents: i64,
    pub absents_prevenus: i64,
    pub retards: i64,
    pub localises: i64,
    pub attendus_ailleurs: i64,
    pub dispenses: i64,
    pub non_pointes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppelV2Dto {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub type_activite: String,
    /// `en_cours`, `neutralise` (séance non assurée) ou un état d'appel validé.
    pub etat: String,
    /// `tous_presents` (défaut) ou `presents_seulement`.
    pub mode_pointage: String,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
    pub delegue_a: Option<Uuid>,
    pub lignes: Vec<LigneAppelV2Dto>,
    pub recapitulatif: RecapAppelDto,
}

/// Corps de `POST /appels/{id}/v2/lignes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerLigneBody {
    pub eleve_id: Uuid,
    /// `present`, `absent`, `retard`, `localise`, `present_dispense`.
    pub etat: String,
    /// Minutes de retard (obligatoire à l'écran pour un retard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lieu_code: Option<String>,
}

/// Corps de `POST /appels/{id}/v2/valider`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValiderAppelBody {
    /// Confirme que les élèves non pointés (mode « présents seulement ») deviennent absents.
    pub confirme_non_pointes: bool,
}

/// Corps de `POST /appels/{id}/v2/corrections`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrigerLigneBody {
    pub eleve_id: Uuid,
    pub etat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lieu_code: Option<String>,
    /// `erreur_saisie`, `billet_entree`, `oubli`, `correction_vie_scolaire`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_texte: Option<String>,
    /// Version de la ligne lue : conflit si elle a changé depuis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_base: Option<i32>,
}

/// Une ligne de l'historique d'un pointage (ajout seul).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoLigneDto {
    pub version: i32,
    pub etat_avant: Option<String>,
    pub etat_apres: String,
    pub minutes: Option<i32>,
    pub lieu_code: Option<String>,
    pub auteur_id: Option<Uuid>,
    pub canal: String,
    pub motif_code: Option<String>,
    pub motif_texte: Option<String>,
    pub cree_le: DateTime<Utc>,
}

/// Incohérence entre l'appel et une situation connue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncoherenceV2Dto {
    pub eleve_id: Uuid,
    pub code: String,
}

/// Situation connue (sans le texte libre).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SituationDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub debut_jour: NaiveDate,
    pub debut_heure: Option<NaiveTime>,
    pub fin_jour: NaiveDate,
    pub fin_heure: Option<NaiveTime>,
    pub heure_arrivee_prevue: Option<NaiveTime>,
    pub source: String,
    pub lieu_code: Option<String>,
    pub etat: String,
}

// ─── Épisodes, qualification, justificatifs (AS3) ────────────────────────────────────────────────

/// Séance d'un épisode (horaires figés au constat).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceEpisodeDto {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub kind: String,
    pub periode: Option<String>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub prevenue: bool,
    pub retiree: bool,
}

/// Épisode d'absence vu par la vie scolaire (`absences.lire_motif`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    /// `ouvert`, `a_qualifier`, `qualifie`, `retire`.
    pub etat: String,
    pub qualification: String,
    pub nature_code: Option<String>,
    pub motif_id: Option<Uuid>,
    pub prevenue: bool,
    pub source: String,
    pub libelle_famille_code: Option<String>,
    pub version: i32,
    pub seances: Vec<SeanceEpisodeDto>,
    pub justificatifs: i64,
    pub propositions_en_attente: i64,
    #[serde(default)]
    pub textes: Vec<String>,
    #[serde(default)]
    pub note_interne: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatEpisodeDto {
    pub etat: String,
    pub qualification: String,
    pub origine: String,
    pub auteur_id: Option<Uuid>,
    pub motif_code: Option<String>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionDto {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub qualification: String,
    pub note: Option<String>,
    pub proposee_par: Uuid,
    pub etat: String,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailEpisodeDto {
    pub episode: EpisodeDto,
    pub historique: Vec<EtatEpisodeDto>,
    pub propositions: Vec<PropositionDto>,
}

/// Vue du professeur (R-AS21) : seulement l'état de régularisation, aucun motif.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeVueProfesseurDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    /// `a_regulariser` ou `regularisee`.
    pub regularisation: String,
}

/// Vue de la famille ou de l'élève majeur : libellé à liste fermée, jamais la note interne (R-AS22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeVueFamilleDto {
    pub id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub prevenue: bool,
    /// Vrai tant que l'épisode attend une décision : la famille peut déposer un justificatif.
    pub a_justifier: bool,
    pub libelle: String,
    pub libelle_code: Option<String>,
}

/// Corps de `POST /assiduite/episodes/{id}/qualification`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifierEpisodeBody {
    /// `reputee_legitime`, `excuse_valable`, `non_valable`, `sans_motif`.
    pub qualification: String,
    pub version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libelle_famille_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_interne: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposition_id: Option<Uuid>,
}

/// Corps de `POST /assiduite/episodes/masse[/apercu]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionMasseBody {
    pub absence_ids: Vec<Uuid>,
    pub qualification: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libelle_famille_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_interne: Option<String>,
    /// Empreinte rendue par l'aperçu ; absente pour demander l'aperçu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empreinte: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuMasseDto {
    pub retenus: Vec<Uuid>,
    /// `(épisode, raison codée)`.
    pub ecartes: Vec<(Uuid, String)>,
    pub empreinte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilanMasseDto {
    pub qualifies: Vec<Uuid>,
}

/// Justificatif tel que rendu : jamais le nom d'origine ni la clé d'objet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JustificatifDto {
    pub id: Uuid,
    pub absence_id: Uuid,
    /// `texte`, `piece`, `vu_non_conserve`.
    pub presente: String,
    pub medical: bool,
    pub mime: Option<String>,
    pub taille_octets: Option<i64>,
    pub depose_qualite: String,
    pub conserve_jusqu_au: Option<NaiveDate>,
    pub cree_le: DateTime<Utc>,
    pub detruit_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub texte: Option<String>,
}

/// Corps de `POST /assiduite/episodes/{id}/justificatifs` (texte seul ou pièce).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeposerJustificatifBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texte: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medical: Option<bool>,
    #[serde(default)]
    pub vu_non_conserve: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienJustificatifDto {
    pub url: String,
    pub expires_at: i64,
}

/// Motif d'établissement (le service ne rend à la famille que les motifs actifs et visibles).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifAsdDto {
    pub id: Uuid,
    pub code: String,
    pub nature_code: String,
    pub libelle: String,
    pub libelle_famille: Option<String>,
    pub visible_famille: bool,
    pub actif: bool,
}

/// Nature de motif (liste fermée dont les cinq motifs de L131-8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatureMotifDto {
    pub code: String,
    pub libelle: String,
    pub legitime_l131_8: bool,
    pub medical: bool,
    pub ordre: i16,
}

// ─── Information des familles (AS5) ──────────────────────────────────────────────────────────────

/// Avis par demi-journée (registre `ent_asd_avis_demi_journee`) : prévu, parti, démenti.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisDemiJourneeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    /// `matin` ou `apres_midi`.
    pub periode: String,
    pub generation: i16,
    /// `en_attente`, `parti`, `annule`, `dementi`… (chaîne du serveur).
    pub statut: String,
    pub cause_code: Option<String>,
    pub avis_id: Uuid,
    pub dementi_avis_id: Option<Uuid>,
    /// État de l'avis dans le moteur (`planifie`, `en_cours`, `termine`, `annule`).
    pub avis_statut: String,
}

/// Réglages d'information des familles (lecture).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamFamilleDto {
    pub delai_consolidation_min: i16,
    pub rythme_relance: String,
    pub avis_demi_journee_actif: bool,
    pub decision_active: bool,
    pub relance_active: bool,
}

/// Épisode « sans réponse » (J+5) ou à proposer « sans motif » (J+8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeSansReponseDto {
    pub absence_id: Uuid,
    pub eleve_id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub derniere_etape: String,
    pub courrier_en_cours: bool,
}

/// Ce que la famille voit de ses contacts avec la vie scolaire (jamais la note, ni l'auteur).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunicationFamilleDto {
    /// `contact` ou `courrier`.
    pub type_communication: String,
    pub canal: String,
    pub cree_le: DateTime<Utc>,
}

// ─── Accueil (AS-216) ────────────────────────────────────────────────────────────────────────────

/// Ce que l'accueil sait d'un élève à l'instant : jamais un motif, jamais un texte libre.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SituationAccueilDto {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub eleve_label: Option<String>,
    /// En sortie, en voyage, en mission… : attendu ailleurs aujourd'hui.
    pub en_sortie: bool,
    /// Une absence ou un retard a été annoncé par la famille.
    pub prevenu: bool,
    pub heure_arrivee_prevue: Option<NaiveTime>,
    /// Localisé à l'infirmerie, à la vie scolaire, au CDI… (lieu codé).
    pub lieu_code: Option<String>,
    /// Absent à l'appel de ce matin.
    pub absent_ce_matin: bool,
}

// ─── Tableau de bord de la vie scolaire (AS6 + AS14b-bis) ──────────────────────────────────────

/// Une ligne de la file du matin (`GET /assiduite/file-du-matin`). Ni motif, ni note interne, ni texte libre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntreeFileDto {
    pub cle: String,
    /// `urgence` · `non_arrivee` · `apres_presence` · `non_jointe` · `sans_nouvelles` · `appel_non_fait` ·
    /// `incoherence`.
    pub categorie: String,
    #[serde(default)]
    pub raisons: Vec<String>,
    /// 1 (le plus urgent) à 9 : calculé par le serveur (`niveau_de`).
    pub niveau: u8,
    pub rang: u32,
    pub eleve_id: Option<Uuid>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub classe: Option<String>,
    pub mineur: Option<bool>,
    pub vigilance: bool,
    pub depuis: Option<NaiveTime>,
    pub minutes_depuis: Option<i64>,
    pub nb_seances: Option<i64>,
    pub absence_id: Option<Uuid>,
    pub alerte_id: Option<Uuid>,
    pub creneau_id: Option<Uuid>,
    pub classe_seance: Option<String>,
    pub intervenant_user_id: Option<Uuid>,
    pub detail_code: Option<String>,
    pub contacts_joints: i64,
    pub contacts_sans_succes: i64,
    pub pris_par: Option<Uuid>,
    pub pris_jusqua: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileDuMatinDto {
    pub jour: NaiveDate,
    pub heure_locale: NaiveTime,
    pub entrees: Vec<EntreeFileDto>,
    #[serde(default)]
    pub compteurs: std::collections::BTreeMap<String, i64>,
}

/// Un élève exclu de cours ce jour, arrivée non confirmée (`GET /assiduite/exclus-du-jour`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExcluDuJourDto {
    pub appel_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub mineur: bool,
    pub depuis: NaiveTime,
    pub minutes_depuis: i64,
    /// Déjà dans le bandeau « Urgent » (au moins 10 minutes sans arrivée).
    pub urgent: bool,
    pub creneau_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExclusDuJourDto {
    pub heure_locale: NaiveTime,
    pub exclus: Vec<ExcluDuJourDto>,
}

/// Disposition d'un panneau : la position dans la liste est le rang.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferencePanneauDto {
    pub panneau: String,
    pub replie: bool,
    pub masque: bool,
}

/// `GET /assiduite/tableau/configuration` : heure de bascule de l'établissement (heure locale) et disposition de
/// l'utilisateur (`None` = automatique).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigTableauDto {
    pub heure_bascule: NaiveTime,
    pub preference: Option<Vec<PreferencePanneauDto>>,
}

/// Corps de `PUT /assiduite/tableau/heure-bascule` et sa réponse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeureBasculeBody {
    pub heure: NaiveTime,
}

/// Corps de `PUT /assiduite/tableau/preference`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferenceTableauBody {
    pub panneaux: Vec<PreferencePanneauDto>,
}

/// Alerte de la vie scolaire (réponse de « je prends » et de « traiter »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub statut: String,
    pub pris_par: Option<Uuid>,
    pub pris_jusqua: Option<DateTime<Utc>>,
    pub cloture_code: Option<String>,
}

/// Réponse de « prévenir maintenant ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatPrevenirDto {
    pub deja_fait: bool,
    pub destinataires: usize,
    pub sans_destinataire: bool,
}

/// Corps de `POST /assiduite/contacts` (trace d'un contact avec la famille ; note interne jamais visible d'elle).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracerContactBody {
    pub eleve_id: Uuid,
    pub absence_id: Option<Uuid>,
    pub canal: String,
    pub issue: String,
}
