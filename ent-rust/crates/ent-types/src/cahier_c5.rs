//! Cahier de textes, lot C5 : **rendu et retour** (`20_CAHIER_DE_TEXTES.md` §10).
//!
//! Contrat JSON des routes `/cahier/c5/*` d'`ent-api`. Le serveur donne des jours et heures **de l'établissement** ;
//! aucun nom de fichier d'élève n'est stocké ni servi (les pièces s'appellent `copie-N.ext`) ; le motif d'une prolongation
//! n'est jamais servi à l'élève (« délai accordé jusqu'au… » seulement) ; les fichiers s'ouvrent par un lien court émis
//! après contrôle des droits.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::cahier::FichierDepotBody;

/// Constats du professeur (R-C12 : des faits, jamais une note).
pub const CONSTAT_RENDU_PAPIER: &str = "rendu_papier";
pub const CONSTAT_NON_RENDU: &str = "non_rendu";
pub const CONSTAT_DISPENSE: &str = "dispense";
pub const CONSTAT_EXCUSE: &str = "excuse";
pub const CONSTAT_ATTENDU: &str = "attendu";

/// Motifs **codés internes** d'une prolongation (jamais affichés à l'élève).
pub const MOTIFS_PROLONGATION: [&str; 4] = ["amenagement", "absence", "technique", "autre"];

/// Niveaux d'un critère de grille.
pub const NIVEAUX_CRITERE: [&str; 4] = ["non_atteint", "partiel", "atteint", "depasse"];

/// Dépôt ou brouillon d'une remise : texte saisi, commentaire à l'intention du professeur, fichiers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotBody {
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub commentaire: Option<String>,
    #[serde(default)]
    pub fichiers: Vec<FichierDepotBody>,
}

/// Dépôt par le professeur au nom d'un élève (copie papier scannée, remise à l'oral).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotAuNomBody {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub fichiers: Vec<FichierDepotBody>,
}

/// Pièce d'une remise ou d'un retour : nom neutre, jamais le nom du fichier d'origine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceC5Dto {
    pub id: Uuid,
    pub nom: String,
    pub mime: Option<String>,
    pub taille: i64,
}

/// Accusé de dépôt : numéro, heure, liste des fichiers, empreinte (R-C9, R-C10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccuseDto {
    pub depot_id: Uuid,
    pub numero: i32,
    pub recu_le: DateTime<Utc>,
    pub recu_jour: NaiveDate,
    pub recu_heure: NaiveTime,
    /// Dépôt fait par le professeur au nom de l'élève : « déposé par M. X le … ».
    pub au_nom_de: bool,
    pub depose_par: Option<String>,
    pub nb_fichiers: i16,
    pub empreinte: String,
    pub texte: Option<String>,
    pub commentaire_eleve: Option<String>,
    pub pieces: Vec<PieceC5Dto>,
    /// Le dépôt courant est le dernier (le professeur voit celui-là par défaut).
    pub courant: bool,
    pub en_retard: bool,
}

/// Brouillon de remise : seul le dépôt compte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrouillonDto {
    pub texte: Option<String>,
    pub commentaire: Option<String>,
    pub pieces: Vec<PieceC5Dto>,
    pub enregistre_le: Option<DateTime<Utc>>,
}

/// Groupe de remise (membres nommés : les membres d'un même groupe se connaissent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeDto {
    pub id: Uuid,
    pub libelle: String,
    pub membres: Vec<MembreGroupeDto>,
    pub complet: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembreGroupeDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

/// Retour vu par l'élève ou un responsable habilité : la note n'est servie que si le professeur l'a choisi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetourLuDto {
    pub retour_id: Uuid,
    pub version: i32,
    /// Retour mis à jour après un premier rendu : « modifié le … » (R-C29).
    pub modifie: bool,
    pub rendu_le: Option<DateTime<Utc>>,
    pub commentaire: String,
    pub criteres: Vec<CritereDto>,
    pub note_valeur: Option<f64>,
    pub note_sur: Option<f64>,
    /// V3c (M2) : statut du résultat publié quand ce n'est pas une note (`absent`, `dispense`, `a_rattraper`,
    /// `non_note`, `non_rendu`, `annule`) : affiché tel quel, jamais converti en « 0 / 20 ».
    #[serde(default)]
    pub note_statut: Option<String>,
    pub pieces: Vec<PieceC5Dto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CritereDto {
    pub libelle: String,
    pub niveau: Option<String>,
}

/// Demande de délai vue de l'élève ou du responsable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelaiLuDto {
    pub id: Uuid,
    pub etat: String,
    pub message: Option<String>,
    pub reponse: Option<String>,
    pub cree_le: DateTime<Utc>,
}

/// Situation d'un élève devant un travail à rendre (lui-même ou son enfant).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemiseLueDto {
    pub travail_id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub rendu: String,
    /// `aucune` (rien déposé), `brouillon`, `deposee`, `a_refaire`.
    pub etat: String,
    /// Statut constaté (`a_faire`, `rendu`, `en_retard`, `non_rendu`, `dispense`, `excuse`, `retour_disponible`).
    pub statut: Option<String>,
    pub echeance_le: DateTime<Utc>,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: NaiveTime,
    /// Une prolongation individuelle s'affiche « délai accordé jusqu'au… », sans motif.
    pub delai_accorde: bool,
    pub cloture_ramassage_le: Option<DateTime<Utc>>,
    pub retard_accepte: bool,
    /// L'élève peut-il déposer maintenant ? Sinon `raison_fermeture` (message clair).
    pub peut_deposer: bool,
    pub raison_fermeture: Option<String>,
    pub formats: Vec<String>,
    pub groupe: Option<GroupeDto>,
    pub remise_par_groupe: bool,
    pub brouillon: Option<BrouillonDto>,
    /// Vide pour un responsable : il ne lit jamais les dépôts avant le retour (R-C37).
    pub depots: Vec<AccuseDto>,
    pub nb_depots: i32,
    pub retour: Option<RetourLuDto>,
    pub demande_delai: Option<DelaiLuDto>,
    pub signalement_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalementBody {
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeDelaiBody {
    #[serde(default)]
    pub message: Option<String>,
}

/// Composition des groupes d'un travail par le professeur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeBody {
    pub libelle: String,
    pub eleve_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupesBody {
    pub groupes: Vec<GroupeBody>,
}

/// Un élève crée un groupe (ou le rejoint) quand le professeur laisse les élèves s'inscrire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeLibreBody {
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub libelle: Option<String>,
}

// ───────────────────────────── côté professeur ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneSuiviDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub groupe: Option<String>,
    pub statut: Option<String>,
    pub remise_id: Option<Uuid>,
    pub dernier_depot_le: Option<DateTime<Utc>>,
    pub nb_depots: i32,
    pub echeance_effective_le: DateTime<Utc>,
    pub prolonge: bool,
    /// Retour : `a_faire`, `brouillon`, `rendu`.
    pub retour: String,
    pub delai_demande_id: Option<Uuid>,
    pub signalement_le: Option<DateTime<Utc>>,
    /// Proposition de constat en lot : ramassage clos sans dépôt, aucun constat posé.
    pub constat_propose: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComptesSuiviDto {
    pub attendus: i32,
    pub rendus: i32,
    pub en_retard: i32,
    pub rendu_papier: i32,
    pub non_rendus: i32,
    pub dispenses: i32,
    pub excuses: i32,
    pub retours_rendus: i32,
    pub retours_a_faire: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiviC5Dto {
    pub travail_id: Uuid,
    pub titre: String,
    pub cours_id: Uuid,
    pub rendu: String,
    pub echeance_le: DateTime<Utc>,
    pub cloture_ramassage_le: DateTime<Utc>,
    pub cloture_atteinte: bool,
    pub remise_par_groupe: bool,
    pub comptes: ComptesSuiviDto,
    pub lignes: Vec<LigneSuiviDto>,
    pub groupes: Vec<GroupeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstatLigneBody {
    pub eleve_id: Uuid,
    pub statut: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstatsBody {
    pub constats: Vec<ConstatLigneBody>,
}

/// Prolongation individuelle : jour et heure de l'établissement, motif codé interne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongationBody {
    pub eleve_ids: Vec<Uuid>,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: NaiveTime,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelaiDecisionBody {
    pub accepter: bool,
    #[serde(default)]
    pub reponse: Option<String>,
    /// Obligatoire si la demande est acceptée.
    #[serde(default)]
    pub echeance_jour: Option<NaiveDate>,
    #[serde(default)]
    pub echeance_heure: Option<NaiveTime>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelaiAttenteDto {
    pub id: Uuid,
    pub travail_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub message: Option<String>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClotureBody {
    pub cloture_jour: NaiveDate,
    pub cloture_heure: NaiveTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetourC5Body {
    #[serde(default)]
    pub commentaire: String,
    #[serde(default)]
    pub criteres: Vec<CritereDto>,
    /// Montrer la note liée à l'élève (sinon « rendre la copie sans la note », CC-37).
    #[serde(default)]
    pub montrer_note: bool,
    #[serde(default)]
    pub note_id: Option<Uuid>,
    /// Copie annotée déposée par le professeur.
    #[serde(default)]
    pub fichiers: Vec<FichierDepotBody>,
    /// Rendre tout de suite (sinon brouillon).
    #[serde(default)]
    pub rendre: bool,
}

/// Retour vu du professeur (brouillon compris) : avertissement doux sur un prénom d'un autre élève (CC-38).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetourProfDto {
    pub retour_id: Uuid,
    pub remise_id: Uuid,
    pub etat: String,
    pub version: i32,
    pub commentaire: String,
    pub criteres: Vec<CritereDto>,
    pub montrer_note: bool,
    pub note_id: Option<Uuid>,
    pub rendu_le: Option<DateTime<Utc>>,
    pub pieces: Vec<PieceC5Dto>,
    pub avertissements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotRenduDto {
    pub rendus: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouvrirBody {
    pub echeance_jour: NaiveDate,
    pub echeance_heure: NaiveTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalementCopieBody {
    /// `cpe`, `direction` ou `protection`.
    pub vers: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelanceBody {
    /// Élèves à relancer ; vide : tous les non-rendus.
    #[serde(default)]
    pub eleve_ids: Vec<Uuid>,
    #[serde(default)]
    pub aux_familles: bool,
    /// Sans confirmation, la route ne fait qu'**aperçu** : rien n'est envoyé (action sensible).
    #[serde(default)]
    pub confirmer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelanceDto {
    pub envoyee: bool,
    pub destinataires: i32,
    pub deja_relances: i32,
    /// Texte prérempli, neutre : jamais un motif ni une note.
    pub texte: String,
}

/// Lien court vers une pièce, émis après contrôle des droits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienPieceDto {
    pub url: String,
    pub expires_at: i64,
}

/// Copie à télécharger : nommée « NOM Prénom – titre », créée à la volée pour l'archive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopieATelechargerDto {
    pub eleve_id: Uuid,
    pub piece_id: Uuid,
    pub nom_archive: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanneBody {
    pub debut_le: DateTime<Utc>,
    pub fin_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanneDto {
    pub panne_id: Uuid,
    pub travaux_prolonges: i32,
    pub prolongation_minutes: i32,
}
