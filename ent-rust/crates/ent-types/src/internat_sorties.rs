//! Types JSON des routes `/internat/*` du socle 41, lot I3 « Absences et sorties » : annonces d'absence des
//! familles, sorties typées, correspondants, départs du vendredi, contrôles de départ et de retour,
//! autorisations annuelles (socle 18).
//!
//! Aucune règle ici : le serveur décide (droits, qualité de titulaire, majorité, désaccords, remise).
//! Les états restent des chaînes. **Aucune donnée de santé** (R-IT6). Un élève protégé (socle 2, E7)
//! n'est jamais nommé à un tiers. Une famille ne voit jamais qui, parmi les autres titulaires, s'oppose :
//! seulement qu'une opposition existe.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────── Types de sortie (S-4105) ─────────────────────────

/// Réglage d'un type de sortie. `autorisation` : `annuelle`, `annuelle_ou_ponctuelle`, `ponctuelle`,
/// `aucune`. `decideur` : `titulaires`, `cpe`, `titulaires_et_cpe`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieTypeDto {
    pub code: String,
    pub libelle: String,
    pub actif: bool,
    pub autorisation: String,
    pub decideur: String,
    pub seul_permis: bool,
    pub eleve_peut_demander: bool,
    pub correspondant_requis: bool,
    pub duree_max_heures: i16,
    pub usage_accord_code: String,
}

/// Modification d'un type (champs absents = inchangés).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SortieTypeSaisieDto {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub actif: Option<bool>,
    #[serde(default)]
    pub autorisation: Option<String>,
    #[serde(default)]
    pub decideur: Option<String>,
    #[serde(default)]
    pub seul_permis: Option<bool>,
    #[serde(default)]
    pub eleve_peut_demander: Option<bool>,
    #[serde(default)]
    pub correspondant_requis: Option<bool>,
    #[serde(default)]
    pub duree_max_heures: Option<i16>,
}

// ───────────────────────── Absences annoncées ─────────────────────────

/// Annonce d'absence d'internat : « Léa ne dormira pas à l'internat mardi soir ». Les nuits sont les dates
/// du soir. Une nuit déjà passée n'est pas une annonce : `regularisation = true` (CIT-37), avec un motif.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbsenceAnnonceDto {
    pub eleve_id: Uuid,
    pub du_nuit: NaiveDate,
    pub au_nuit: NaiveDate,
    /// Personne qui récupère l'élève (socle 4 : titulaire ou personne autorisée à récupérer).
    #[serde(default)]
    pub recupere_par_personne_id: Option<Uuid>,
    #[serde(default)]
    pub retour_prevu: Option<DateTime<Utc>>,
    #[serde(default)]
    pub regularisation: bool,
    /// Motif court (300 caractères, aucune donnée de santé) : exigé pour une régularisation.
    #[serde(default)]
    pub motif_texte: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbsenceDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub du_nuit: NaiveDate,
    pub au_nuit: NaiveDate,
    /// `annonce` ou `regularisation`.
    pub mode: String,
    /// `demandee`, `annoncee`, `acceptee`, `refusee`, `bloquee`, `regularisee`, `annulee`.
    pub statut: String,
    /// Annonce faite après l'heure limite : le CPE décide (aucun effet avant).
    pub tardive: bool,
    /// `titulaire`, `eleve_majeur`, `personnel`.
    pub annonce_qualite: String,
    pub recupere_par_personne_id: Option<Uuid>,
    pub retour_prevu: Option<DateTime<Utc>>,
    pub decide_le: Option<DateTime<Utc>>,
    /// Motif du refus ou du blocage, lu par la famille.
    pub motif_refus_texte: Option<String>,
    /// Un titulaire s'oppose (l'identité n'est jamais rendue à la famille).
    pub opposition: bool,
    /// Heure du départ contrôlé au portail (C-1), s'il a eu lieu.
    #[serde(default)]
    pub depart_effectif: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

/// Décision du CPE. `decision` : `accepter`, `refuser` (motif lu par la famille), `regulariser`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionDto {
    pub decision: String,
    #[serde(default)]
    pub motif_texte: Option<String>,
}

/// Opposition d'un titulaire à une absence ou à une sortie : le plus restrictif l'emporte tant que la
/// direction n'a pas tranché.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OppositionDto {
    #[serde(default)]
    pub motif_texte: Option<String>,
}

/// Décision de la direction sur un désaccord : `maintenir` (la demande est refusée) ou `lever` (elle est
/// rétablie, avec trace).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrancheDto {
    pub decision: String,
    #[serde(default)]
    pub motif_texte: Option<String>,
}

// ───────────────────────── Sorties individuelles ─────────────────────────

/// Demande (ou annonce) d'une sortie. `type_code` : `mercredi`, `soiree`, `nuit_correspondant`,
/// `week_end`, `retour_tardif`, `exceptionnelle`. Seul : `seul = true` ; sinon une personne (ou le
/// correspondant) qui récupère.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieDemandeDto {
    pub eleve_id: Uuid,
    pub type_code: String,
    pub debut: DateTime<Utc>,
    pub fin_prevue: DateTime<Utc>,
    #[serde(default)]
    pub seul: bool,
    #[serde(default)]
    pub avec_personne_id: Option<Uuid>,
    #[serde(default)]
    pub correspondant_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub type_code: String,
    pub debut: DateTime<Utc>,
    pub fin_prevue: DateTime<Utc>,
    /// Retour attendu à jour (retard annoncé compris).
    pub fin_ajustee: DateTime<Utc>,
    pub seul: bool,
    pub avec_personne_id: Option<Uuid>,
    pub correspondant_id: Option<Uuid>,
    /// `demandee`, `autorisee`, `refusee`, `bloquee`, `suspendue`, `partie`, `rentree`, `annulee`, `expiree`.
    pub statut: String,
    pub accord_titulaire: bool,
    pub decision_cpe: bool,
    pub par_autorisation_annuelle: bool,
    pub depart_effectif: Option<DateTime<Utc>>,
    pub retour_effectif: Option<DateTime<Utc>>,
    /// Un titulaire s'oppose (l'identité n'est jamais rendue à la famille).
    pub opposition: bool,
    /// Motif du refus ou du maintien du blocage, lu par la famille (texte d'un CPE ou de la direction seulement).
    #[serde(default)]
    pub motif_refus_texte: Option<String>,
    /// Retour demandé en retard qui ferait couvrir la nuit à une sortie qui ne la couvrait pas (E-2) : sans effet
    /// tant que le CPE n'a pas décidé (`accepter_retard` ou `refuser_retard`).
    #[serde(default)]
    pub retard_demande: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

/// Décision du CPE ou de la direction sur une sortie : `autoriser` ou `refuser` (sortie demandée), `accepter_retard`
/// ou `refuser_retard` (retard en attente, E-2).
pub type SortieDecisionDto = DecisionDto;

/// Réponse d'un titulaire (ou de l'élève majeur) à une demande de sortie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccordTitulaireDto {
    /// `true` : accord ; `false` : refus (le plus restrictif l'emporte).
    pub accord: bool,
    #[serde(default)]
    pub motif_texte: Option<String>,
    /// Personne qui récupère l'élève (socle 4), précisée à l'accord pour une demande de l'élève.
    #[serde(default)]
    pub avec_personne_id: Option<Uuid>,
}

/// Qui récupère l'élève (ou départ seul) : le titulaire ou le CPE le précise tant que la sortie n'est pas partie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersonneSortieDto {
    #[serde(default)]
    pub avec_personne_id: Option<Uuid>,
    #[serde(default)]
    pub seul: bool,
}

/// Retard de retour annoncé (train supprimé, grève) : le retour attendu est décalé, aucun cas ne s'ouvre
/// tant que le délai annoncé court (CIT-23).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetardAnnonceDto {
    pub nouveau_retour: DateTime<Utc>,
    /// `train` ou `autre`.
    #[serde(default)]
    pub motif_code: Option<String>,
}

/// Contrôle d'un départ : pointage par le surveillant ou l'accueil. `remis_a_personne_id` pour un départ
/// accompagné (une personne autorisée aujourd'hui, R-IT23) ; absent pour un départ seul.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControleDepartDto {
    #[serde(default)]
    pub remis_a_personne_id: Option<Uuid>,
    /// Heure du geste (défaut : maintenant).
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControleRetourDto {
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
}

// ───────────────────────── Départs, remise, soirée ─────────────────────────

/// Résultat de l'établissement de l'attendu de départ d'un jour (INT-157).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepartsPrevusDto {
    pub jour: NaiveDate,
    /// Sorties de week-end créées par ce calcul.
    pub crees: i32,
    /// Départs déjà couverts par une sortie ou une absence (rien à créer).
    pub deja_couverts: i32,
    /// Départs créés sans personne désignée (la famille ou le CPE la précise avant la remise).
    pub sans_personne: i32,
}

/// Personne vue de l'accueil pour la remise : nom, prénom et relation, jamais de coordonnée ni d'adresse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersonneRemiseDto {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub relation: String,
}

/// Une ligne de la liste de remise du jour (INT-167) : qui part, à quelle heure, et qui peut le prendre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListeRemiseLigneDto {
    /// La sortie concernée ; absent pour le départ d'une absence annoncée (`absence_id`, type `absence`).
    pub sortie_id: Option<Uuid>,
    /// L'absence annoncée dont le départ se contrôle au portail (C-1).
    #[serde(default)]
    pub absence_id: Option<Uuid>,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub type_code: String,
    pub debut: DateTime<Utc>,
    pub fin_ajustee: DateTime<Utc>,
    pub seul: bool,
    pub statut: String,
    pub personne_prevue: Option<PersonneRemiseDto>,
    /// Personnes autorisées aujourd'hui à prendre l'élève, hors personnes visées par une interdiction.
    pub personnes_autorisees: Vec<PersonneRemiseDto>,
    /// Consigne rouge : personnes à qui l'élève ne doit PAS être remis (nom seulement, aucun motif).
    pub personnes_interdites: Vec<PersonneRemiseDto>,
    /// Une interdiction sans personne identifiée est en vigueur : aucune remise accompagnée sans la direction.
    pub interdiction_non_nominative: bool,
}

/// Élève en sortie courte (mercredi, soirée) à la nuit donnée : il reste attendu ; le surveillant le pointe
/// « en activité » avec son retour attendu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieDuSoirDto {
    pub sortie_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub type_code: String,
    pub statut: String,
    pub fin_ajustee: DateTime<Utc>,
}

// ───────────────────────── Correspondants ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrespondantSaisieDto {
    pub eleve_id: Uuid,
    /// Personne autorisée à récupérer l'élève (socle 4, statut validé).
    pub personne_id: Uuid,
    #[serde(default)]
    pub lundi: bool,
    #[serde(default)]
    pub mardi: bool,
    #[serde(default)]
    pub mercredi: bool,
    #[serde(default)]
    pub jeudi: bool,
    #[serde(default)]
    pub vendredi: bool,
    #[serde(default)]
    pub samedi: bool,
    #[serde(default)]
    pub dimanche: bool,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrespondantDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub lundi: bool,
    pub mardi: bool,
    pub mercredi: bool,
    pub jeudi: bool,
    pub vendredi: bool,
    pub samedi: bool,
    pub dimanche: bool,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// `valide`, `suspendu`, `termine`.
    pub statut: String,
}

/// Suspension d'un correspondant (CIT-33) : `ne_repond_plus`, `demande_titulaire`, `interdiction` ou `autre`
/// sont rangés dans le code du fait ; ici le texte court est facultatif.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CorrespondantSuspensionDto {
    #[serde(default)]
    pub motif_texte: Option<String>,
}

// ───────────────────────── Autorisations annuelles (socle 18) ─────────────────────────

/// État de l'autorisation annuelle d'un élève pour un type de sortie : `verdict` du socle 18 (`autorise`,
/// `en_attente`, `refuse`, `desaccord`, `expire`, `inconnu`…). Jamais `autorise` par défaut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutorisationTypeDto {
    pub type_code: String,
    pub usage_code: String,
    pub verdict: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutorisationsEleveDto {
    pub eleve_id: Uuid,
    pub types: Vec<AutorisationTypeDto>,
}

/// Demande des autorisations annuelles de sortie pour un ensemble d'élèves (INT-162) : le socle 18 les
/// recueille ; le 41 n'émet que la demande.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutorisationsDemandeDto {
    pub eleve_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutorisationsDemandeesDto {
    pub demandes: i32,
}

/// Révocation d'une autorisation annuelle par un titulaire (INT-163) : effet immédiat, sorties à venir
/// suspendues.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RevocationDto {
    pub eleve_id: Uuid,
    /// Usages retirés (`sortie_mercredi`, …) ; absent = tous.
    #[serde(default)]
    pub usages: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RevocationResultatDto {
    pub sorties_suspendues: i32,
}

/// Tâche de fond : ouvertures de cas par le système (retard de retour, non récupéré).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VeilleSortiesDto {
    pub cas_retard_retour: i32,
    pub cas_non_recupere: i32,
    pub sorties_expirees: i32,
    /// Tiers autorisés dont l'échéance approche : alerte émise (une seule fois par tiers).
    #[serde(default)]
    pub tiers_a_echeance: i32,
}

// ───────────────────────── Historique ─────────────────────────

/// Un fait de l'historique d'une absence, d'une sortie ou d'un correspondant (ajout seul : qui, quand, quoi). La
/// famille ne voit ni l'auteur ni le texte libre d'un autre titulaire (`motif_texte` n'est rendu à la famille que
/// pour un refus ou un maintien du blocage, décisions de l'établissement) ; le personnel de l'internat voit tout, sans
/// jamais un nom (identifiants seulement).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaitDto {
    pub acte: String,
    pub qualite: String,
    pub a: DateTime<Utc>,
    pub par_user_id: Option<Uuid>,
    pub motif_code: Option<String>,
    pub motif_texte: Option<String>,
    pub personne_id: Option<Uuid>,
    pub jusqu_a: Option<DateTime<Utc>>,
}

// ───────────────────────── Tiers autorisé à récupérer (lot I3, suites, M3) ─────────────────────────

/// Pose du statut « tiers autorisé à récupérer » par le personnel (CPE, vie scolaire, direction), à la demande **écrite**
/// d'un titulaire de l'autorité parentale. Dates `du` / `au` obligatoires (un an au plus), motif obligatoire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TiersPoseDto {
    pub eleve_id: Uuid,
    pub personne_id: Uuid,
    pub demande_par_personne_id: Uuid,
    pub demande_ecrite_le: NaiveDate,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub motif_texte: String,
}

/// Un tiers autorisé. La famille voit la personne, les dates, l'état et si le tiers est remettable aujourd'hui ; jamais le
/// motif, ni qui a demandé, ni qui s'oppose (réservés au personnel de l'internat).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TiersDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// Personnel : `actif`, `conteste` (désaccord : porte fermée jusqu'à la direction), `revoque`. **Famille** : `actif`
    /// ou `en_cours_d_examen` — libellé neutre, identique quelle que soit la cause de la fermeture (opposition d'un autre
    /// titulaire, décision en cours, demandeur ou lien écarté, interdiction) : la famille ne déduit rien d'une cause.
    pub statut: String,
    /// La porte est ouverte aujourd'hui (statut actif, dates, aucune interdiction, demandeur toujours titulaire).
    pub remettable_aujourdhui: bool,
    /// Le statut arrive à échéance dans les quinze jours (alerte avant échéance ; renouvellement = nouvelle pose).
    #[serde(default)]
    pub echeance_proche: bool,
    /// Personnel disposant de `internat.sorties.decider` seulement.
    #[serde(default)]
    pub demande_par_personne_id: Option<Uuid>,
    #[serde(default)]
    pub motif_texte: Option<String>,
}

/// Reprise de la demande par un autre titulaire (le demandeur n'a plus le droit d'agir).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TiersRepriseDto {
    pub demande_par_personne_id: Uuid,
    #[serde(default)]
    pub motif_texte: Option<String>,
}
