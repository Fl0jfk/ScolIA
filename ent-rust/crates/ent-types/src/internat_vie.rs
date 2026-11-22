//! Types JSON des routes `/internat/*` du socle 41, lot I4 « Vie quotidienne » : sorties de groupe, liaison codée,
//! événements de nuit, temps du soir (études, activités), préoccupation.
//!
//! Aucune règle ici : le serveur décide (droits, périmètre, qualité de titulaire, interdictions, désaccords).
//! Les états et les codes restent des chaînes. **Aucune donnée de santé** (R-IT6) ; un élève protégé (socle 2, E7)
//! n'est jamais nommé à un tiers ; une famille ne voit jamais un autre élève (R-IT32).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────── Sorties de groupe ─────────────────────────

/// Création d'une sortie de groupe (brouillon). Les accompagnateurs sont des comptes du personnel (R-IT25).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeSaisieDto {
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrants: Vec<Uuid>,
    #[serde(default)]
    pub eleves: Vec<Uuid>,
}

/// Élève écarté d'une invitation (jamais en silence), avec le motif codé : `plus_interne`, `inconnu`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EcarteDto {
    pub eleve_id: Uuid,
    pub motif: String,
}

/// Un participant tel que le personnel le voit. `nom` et `prenom` sont absents pour un élève protégé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantGroupeDto {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    /// `invite`, `en_attente`, `autorise`, `refuse`, `non_autorise`, `parti`, `rentre`, `retire`.
    pub etat: String,
    /// `titulaire`, `eleve_majeur`, `annuelle`, `hors_ligne`.
    #[serde(default)]
    pub reponse_qualite: Option<String>,
    #[serde(default)]
    pub reponse_le: Option<DateTime<Utc>>,
    /// Code du motif d'un refus ou d'un blocage ; jamais « interdiction » détaillée à la famille (voir le contrat).
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub parti_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub rentre_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeDto {
    pub id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `brouillon`, `soumise`, `acceptee`, `refusee`, `annulee`, `close`.
    pub statut: String,
    pub cree_par_user_id: Uuid,
    #[serde(default)]
    pub decide_par_user_id: Option<Uuid>,
    #[serde(default)]
    pub decide_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub decision_motif_code: Option<String>,
    #[serde(default)]
    pub annule_motif_code: Option<String>,
    #[serde(default)]
    pub bilan_code: Option<String>,
    #[serde(default)]
    pub modele_id: Option<Uuid>,
    pub encadrants: Vec<Uuid>,
    pub participants: Vec<ParticipantGroupeDto>,
    /// Élèves écartés à la création ou à l'ajout (seulement dans la réponse à cet appel).
    #[serde(default)]
    pub ecartes: Vec<EcarteDto>,
}

/// Ligne de liste (sans participants nominatifs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeResumeDto {
    pub id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub statut: String,
    pub nb_participants: i64,
    pub nb_autorises: i64,
    pub nb_en_attente: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevesAjoutDto {
    pub eleves: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncadrantAjoutDto {
    pub user_id: Uuid,
}

/// Décision de la direction : `accepte` ; un refus porte un motif codé (`securite`, `encadrement`, `calendrier`,
/// `reglement`, `autre`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionGroupeDto {
    pub accepte: bool,
    #[serde(default)]
    pub motif_code: Option<String>,
}

/// Annulation : motif codé (`meteo`, `encadrement`, `organisateur`, `securite`, `autre`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnulationGroupeDto {
    pub motif_code: String,
}

/// Réponse d'un titulaire (ou de l'élève majeur) pour UN élève.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReponseGroupeDto {
    pub accord: bool,
}

/// Accord reçu hors ligne (R-IT24) : la personne qui l'a donné (un titulaire), la date de réception, le canal
/// (`telephone`, `papier`, `en_personne`) ; la personne qui l'a reçu est le compte appelant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccordHorsLigneDto {
    pub donne_par_personne_id: Uuid,
    pub recu_le: DateTime<Utc>,
    pub canal: String,
    #[serde(default)]
    pub piece_id: Option<Uuid>,
}

/// Appel de départ ou de retour : les élèves effectivement pointés.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelGroupeDto {
    pub eleves: Vec<Uuid>,
}

/// Résultat par élève d'un appel de départ : `ok` ou le motif codé du refus (`non_autorise`, `accord_tombe`,
/// `pas_invite`, `deja_parti`, `pas_parti`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatAppelDto {
    pub eleve_id: Uuid,
    pub resultat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClotureGroupeDto {
    /// `sans_incident`, `retard`, `incident`.
    pub bilan_code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DuplicationGroupeDto {
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

/// Ce que voit une famille (ou l'élève lui-même) d'une sortie de groupe : seulement SON enfant, jamais un autre
/// participant, jamais le nombre d'inscrits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeFamilleDto {
    pub sortie_id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub statut: String,
    /// `en_attente` : une réponse est attendue ; `autorise` ; `refuse` ; `non_autorise` (sans cause) ; `parti` ; `rentre`.
    pub etat: String,
    pub peut_repondre: bool,
}

// ───────────────────────── Liaison codée ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeDto {
    pub code: String,
    pub libelle: String,
    pub pour_le_jour: bool,
    pub valorisation: bool,
    pub ordre: i16,
    pub actif: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeSaisieDto {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub pour_le_jour: Option<bool>,
    #[serde(default)]
    pub actif: Option<bool>,
    #[serde(default)]
    pub ordre: Option<i16>,
}

/// Création d'un code propre à l'établissement (INT-247) ; jamais une catégorie de santé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeCreationDto {
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub pour_le_jour: bool,
}

/// Écriture d'un fait de liaison. `nuit` : date du soir (aujourd'hui par défaut, jamais future).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonSaisieDto {
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub nuit: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    pub code: String,
    pub libelle: String,
    /// Absent pour la vie scolaire de jour (qui ne voit jamais le texte libre).
    #[serde(default)]
    pub texte: Option<String>,
    pub nuit: NaiveDate,
    pub par_user_id: Uuid,
    pub pour_le_jour: bool,
    pub masque: bool,
    pub a_revoir: bool,
    pub signale_pp: bool,
    pub cree_le: DateTime<Utc>,
}

/// Geste du CPE sur un fait : `masquer` (motif obligatoire), `a_revoir`, `lever_a_revoir`, `signaler_pp`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonGesteDto {
    pub geste: String,
    #[serde(default)]
    pub motif_code: Option<String>,
}

// ───────────────────────── Événements de nuit ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantEvenementSaisieDto {
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
    /// `auteur_presume`, `victime`, `temoin`, `present` (un adulte n'est que `temoin` ou `present`).
    pub role: String,
}

/// Déclaration d'un événement. `nature` : `incident`, `comportement`, `intrusion`, `alarme`, `malaise`,
/// `degradation`, `objet_dangereux`. `niveau` : `information`, `important`, `critique`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementSaisieDto {
    pub nature: String,
    #[serde(default)]
    pub niveau: Option<String>,
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub chambre_id: Option<Uuid>,
    #[serde(default)]
    pub mesures_code: Option<String>,
    #[serde(default)]
    pub participants: Vec<ParticipantEvenementSaisieDto>,
    #[serde(default)]
    pub complement: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantEvenementDto {
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    #[serde(default)]
    pub user_id: Option<Uuid>,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComplementDto {
    pub id: Uuid,
    pub texte: String,
    pub par_user_id: Uuid,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementDto {
    pub id: Uuid,
    pub nature: String,
    pub niveau: String,
    pub a: DateTime<Utc>,
    pub nuit: NaiveDate,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub chambre_id: Option<Uuid>,
    #[serde(default)]
    pub mesures_code: Option<String>,
    pub declare_par_user_id: Uuid,
    #[serde(default)]
    pub transmis_13_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub intervention_demandee_le: Option<DateTime<Utc>>,
    pub participants: Vec<ParticipantEvenementDto>,
    pub complements: Vec<ComplementDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComplementSaisieDto {
    pub texte: String,
}

// ───────────────────────── Temps du soir ─────────────────────────

/// Création d'une étude ou d'une activité du soir. `nature` : `etude`, `activite`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempsSaisieDto {
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrant_user_id: Option<Uuid>,
    #[serde(default)]
    pub places: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempsDto {
    pub id: Uuid,
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrant_user_id: Option<Uuid>,
    #[serde(default)]
    pub places: Option<i16>,
    pub statut: String,
    pub nb_inscrits: i64,
    /// Seulement pour le personnel : les inscrits (jamais montrés à une famille).
    #[serde(default)]
    pub inscrits: Vec<Uuid>,
}

/// Planning d'un élève (lu par lui, ses titulaires ou le personnel) : jamais le nom d'un autre élève.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningEleveDto {
    pub eleve_id: Uuid,
    pub etudes_et_activites: Vec<PlanningTempsDto>,
    pub sorties_groupe: Vec<SortieGroupeFamilleDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningTempsDto {
    pub temps_id: Uuid,
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InscriptionTempsDto {
    pub eleve_id: Uuid,
}

// ───────────────────────── Préoccupation ─────────────────────────

/// Ouverture du circuit du socle 36 : aucun contenu (R-IT29).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreoccupationDto {
    pub eleve_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreoccupationAccuseDto {
    pub id: Uuid,
    pub a: DateTime<Utc>,
}

// ───────────────────────── Vigilance et référent (INT-257) ─────────────────────────

/// Pose d'une vigilance codée (`suivi_rapproche`, `premier_mois`, `accompagner_coucher`), sans texte. `du` : aujourd'hui
/// par défaut ; `au` : sans fin tant qu'elle n'est pas levée. Le référent est un compte du personnel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilanceSaisieDto {
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub referent_user_id: Option<Uuid>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilanceDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub referent_user_id: Option<Uuid>,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    pub revue_le: DateTime<Utc>,
    /// Vrai quand la dernière revue date de plus de 7 jours : la vigilance attend sa revue hebdomadaire.
    pub a_reviser: bool,
    /// Vrai tant que la vigilance est en vigueur au jour de la lecture.
    pub active: bool,
}
