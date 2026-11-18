//! Types JSON des routes `/internat/*` du socle 41, lot I2 « La nuit » : attendu, appels, pointages,
//! cas et procédure, transmission du matin.
//!
//! Aucune règle ici : le serveur décide (droits, périmètre, R-IT14, R-IT16…). Les états restent des
//! chaînes. **Aucune donnée de santé** (R-IT6) : une marque « à l'infirmerie » renvoie vers le
//! socle 34 sans rien dire du motif. Un élève protégé (socle 2, E7) n'est jamais nommé à un tiers :
//! sa ligne porte `protege = true`, sans nom, avec son lit pour le retrouver.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────── Attendu ─────────────────────────

/// Une ligne de l'attendu d'une nuit. `statut` : `attendu`, `ajoute` (nuit exceptionnelle), `retire`
/// (avec `motif_code`) ou `non_attendu` (hébergé mais hors rythme).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttenduLigneDto {
    pub eleve_id: Uuid,
    /// Nom d'usage et prénom ; `None` pour un élève protégé caché à ce lecteur.
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub chambre_code: Option<String>,
    pub lit_numero: Option<i16>,
    pub statut: String,
    pub motif_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttenduDto {
    pub nuit: NaiveDate,
    /// Heure du dernier calcul (affichée sur la vue d'appel).
    pub calcule_le: DateTime<Utc>,
    pub lignes: Vec<AttenduLigneDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NuitExceptionnelleSaisieDto {
    pub eleve_id: Uuid,
    pub nuit: NaiveDate,
    /// `examen`, `accueil_weekend`, `intemperies`, `delai_accorde`, `decision_direction`, `autre`.
    pub motif_code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NuitExceptionnelleDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nuit: NaiveDate,
    pub motif_code: String,
    pub annulee: bool,
}

/// Retrait de l'attendu : adaptateur des autres socles et saisie du CPE (motif codé, jamais en clair).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetraitSaisieDto {
    pub eleve_id: Uuid,
    pub du_nuit: NaiveDate,
    pub au_nuit: NaiveDate,
    /// `absence_annoncee`, `sortie`, `mobilite`, `stage`, `sejour_scolaire`, `exclusion_internat`,
    /// `fermeture`, `sante`.
    pub motif_code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetraitDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub du_nuit: NaiveDate,
    pub au_nuit: NaiveDate,
    pub motif_code: String,
    pub source: String,
    pub annule: bool,
}

// ───────────────────────── Listes ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListeSaisieDto {
    pub libelle: String,
    #[serde(default)]
    pub chambre_ids: Vec<Uuid>,
    #[serde(default)]
    pub confie_a_user_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListeDto {
    pub id: Uuid,
    pub libelle: String,
    pub chambre_ids: Vec<Uuid>,
    pub confie_a_user_id: Option<Uuid>,
    pub actif: bool,
}

// ───────────────────────── Appels ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelOuvertureDto {
    /// Nuit (date du soir) ; défaut : la nuit en cours dans le fuseau de l'établissement.
    #[serde(default)]
    pub nuit: Option<NaiveDate>,
    /// `arrivee`, `etude`, `soir`, `extinction`, `ronde`, `matin`, `rassemblement`.
    pub type_appel: String,
    /// `internat`, `batiment`, `zone`, `liste`.
    pub perimetre: String,
    #[serde(default)]
    pub batiment_id: Option<Uuid>,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub liste_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelDto {
    pub id: Uuid,
    pub nuit: NaiveDate,
    pub type_appel: String,
    pub perimetre: String,
    pub batiment_id: Option<Uuid>,
    pub zone_id: Option<Uuid>,
    pub liste_id: Option<Uuid>,
    /// `ouvert`, `clos`, `valide`.
    pub etat: String,
    /// Appel saisi après la nuit (CIT-48) : tracé comme tel.
    pub a_posteriori: bool,
    pub ouvert_le: DateTime<Utc>,
    pub clos_le: Option<DateTime<Utc>>,
    pub valide_le: Option<DateTime<Utc>>,
    pub nb_attendus: i32,
    pub nb_ecarts: Option<i32>,
}

/// Ligne de la vue d'appel : la liste figée à l'ouverture et la valeur courante du pointage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelLigneDto {
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub chambre_code: Option<String>,
    pub lit_numero: Option<i16>,
    pub statut: String,
    pub motif_code: Option<String>,
    /// Marque courante (le fait le plus récent), `None` tant que l'élève n'est pas pointé.
    pub marque: Option<String>,
    pub marque_a: Option<DateTime<Utc>>,
    pub marque_par_user_id: Option<Uuid>,
    pub retour_attendu: Option<DateTime<Utc>>,
    /// Deux surveillants ont pointé le même élève en sens contraire à moins de deux minutes de réception
    /// (CIT-03, M8) : la fenêtre se mesure sur l'heure de réception serveur, jamais sur l'heure client.
    pub conflit: bool,
    /// Marques contradictoires de la fenêtre (tous les faits distincts, pas seulement l'avant-dernier).
    #[serde(default)]
    pub marques_en_conflit: Vec<String>,
    /// Cet élève a une marque « à l'infirmerie » : renvoi vers l'infirmerie (socle 34), sans motif.
    pub renvoi_infirmerie: bool,
    pub corrige: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelVueDto {
    pub appel: AppelDto,
    /// Heure du dernier calcul de la vue (l'attendu figé reste celui de l'ouverture).
    pub calcule_le: DateTime<Utc>,
    pub lignes: Vec<AppelLigneDto>,
    pub nb_pointes: i32,
    /// Élèves dont l'attendu actuel diffère de la liste figée (nouveau retrait, nouvelle nuit exceptionnelle).
    pub ecarts_avec_attendu_actuel: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointageSaisieDto {
    pub eleve_id: Uuid,
    /// `present`, `absent`, `activite`, `infirmerie`, `parti`, `non_attendu_present`, `vu`, `retard`.
    pub marque: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub retour_attendu: Option<DateTime<Utc>>,
    /// Heure du geste (hors ligne : horodatage local) ; défaut : maintenant.
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
    #[serde(default)]
    pub appareil_id: Option<Uuid>,
    /// Obligatoire sur un appel clos ou validé (R-IT17) : `oubli`, `erreur_saisie`,
    /// `information_tardive`, `rattrapage`, `autre`.
    #[serde(default)]
    pub motif_correction_code: Option<String>,
    #[serde(default)]
    pub motif_correction_texte: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToutPresentSaisieDto {
    pub chambre_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointageResultatDto {
    pub fait_id: Uuid,
    pub eleve_id: Uuid,
    pub marque: String,
    pub conflit: bool,
    pub renvoi_infirmerie: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClotureResultatDto {
    pub appel: AppelDto,
    /// Cas ouverts à la clôture (absent non couvert, retour tardif).
    pub cas_ouverts: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidationResultatDto {
    pub appel: AppelDto,
    /// `U1` (aucun écart) ou `U3` (un cas est ouvert) : niveau de l'avis (R-IT34).
    pub urgence: String,
    pub nb_ecarts: i32,
    pub cas_ouverts: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoriqueFaitDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub marque: String,
    pub note: String,
    pub a: DateTime<Utc>,
    pub recu_le: DateTime<Utc>,
    pub par_user_id: Uuid,
    pub motif_correction_code: Option<String>,
    pub motif_correction_texte: Option<String>,
}

// ───────────────────────── Cas et procédure ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub protege: bool,
    pub nuit: NaiveDate,
    /// `non_retrouve`, `non_rentre`, `non_recupere`, `retard_retour`.
    pub nature: String,
    pub ouvert_le: DateTime<Utc>,
    pub issue: Option<String>,
    pub issue_le: Option<DateTime<Utc>>,
    pub transmis_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EtapeProcedureDto {
    pub id: Uuid,
    pub rang: i16,
    pub code: String,
    pub libelle: String,
    pub delai_minutes: i16,
    /// `astreinte`, `titulaires`, `direction`, `secours`, `aucun`.
    pub appelle: String,
    /// Échéance de l'étape (ouverture + délai) : le minuteur du surveillant.
    pub echeance: DateTime<Utc>,
    pub faite_le: Option<DateTime<Utc>>,
    pub faite_par_user_id: Option<Uuid>,
}

/// Numéro à composer : jamais d'adresse, jamais un numéro non autorisé (R-IT26).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumeroDto {
    pub personne_id: Uuid,
    pub libelle: String,
    pub numero: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasVueDto {
    pub cas: CasDto,
    pub procedure_version: i32,
    pub etapes: Vec<EtapeProcedureDto>,
    /// Lieux déjà vérifiés (`chambre`, `sanitaires`, `etude`, `infirmerie`, `autre_zone`).
    pub lieux_verifies: Vec<String>,
    /// Numéros des titulaires joignables (socle 4) — vide si aucun n'est autorisé.
    pub numeros_titulaires: Vec<NumeroDto>,
    /// `true` quand aucun numéro n'est rendu parce que l'élève est protégé (E3, décision pessimiste du
    /// pilote) : la vue indique de passer par la direction ou le CPE d'astreinte.
    #[serde(default)]
    pub numeros_masques: bool,
    /// Le CPE d'astreinte vient du socle 6 ; tant qu'il n'est pas branché, `false`.
    pub astreinte_branchee: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasEtapeSaisieDto {
    /// Étape de la procédure faite…
    #[serde(default)]
    pub etape_id: Option<Uuid>,
    /// …ou lieu vérifié (`chambre`, `sanitaires`, `etude`, `infirmerie`, `autre_zone`, `exterieur`).
    #[serde(default)]
    pub lieu_code: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Heure du fait ; défaut : maintenant.
    #[serde(default)]
    pub fait_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasIssueSaisieDto {
    /// `retrouve`, `rentre_famille`, `autorite_saisie`, `annule_erreur`.
    pub issue: String,
    /// Où il a été retrouvé : `chambre`, `sanitaires`, `etude`, `infirmerie`, `autre_zone`, `exterieur`.
    #[serde(default)]
    pub lieu_code: Option<String>,
    /// Personne du socle 4 qui a confirmé le retour en famille.
    #[serde(default)]
    pub confirme_par_personne_id: Option<Uuid>,
    #[serde(default)]
    pub numero_intervention: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcedureEtapeSaisieDto {
    pub code: String,
    pub libelle: String,
    pub delai_minutes: i16,
    pub appelle: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcedureSaisieDto {
    pub etapes: Vec<ProcedureEtapeSaisieDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcedureDto {
    pub version: i32,
    pub etapes: Vec<ProcedureEtapeSaisieDto>,
}

// ───────────────────────── Transmission du matin ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransmissionDto {
    pub nuit: NaiveDate,
    /// Version de la transmission : 1 = matin ; n > 1 = rectificative (M3).
    #[serde(default = "version_un")]
    pub version: i32,
    pub transmis_le: DateTime<Utc>,
    pub nb_attendus: i32,
    pub nb_presents: i32,
    pub nb_absents: i32,
    pub nb_cas: i32,
    /// `false` si la nuit avait déjà été transmise avec les mêmes chiffres (la route est idempotente).
    pub nouvelle: bool,
}

fn version_un() -> i32 {
    1
}

/// Corps (facultatif) de la transmission : le motif est exigé quand la transmission est rectificative.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TransmissionSaisieDto {
    #[serde(default)]
    pub motif_rectification: Option<String>,
}

/// Effectif de nuit pour la restauration (socle 40) : des compteurs, aucun nom.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectifRepasDto {
    pub nuit: NaiveDate,
    /// Dîner de la nuit : internes attendus ce soir.
    pub diner: i32,
    /// Petit-déjeuner du lendemain : attendus moins ceux déclarés absents ou partis au matin.
    pub petit_dejeuner: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RappelsDto {
    pub surveillant: Vec<AppelRappelDto>,
    pub cpe: Vec<AppelRappelDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppelRappelDto {
    pub nuit: NaiveDate,
    pub type_appel: String,
}

/// Une ligne du journal des lectures sensibles (R-IT38).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LectureSensibleDto {
    pub user_id: Uuid,
    pub objet: String,
    pub eleve_id: Option<Uuid>,
    pub appel_id: Option<Uuid>,
    /// Élèves concernés par la lecture (attendu, appel), quand elle en concerne plusieurs.
    #[serde(default)]
    pub eleve_ids: Vec<Uuid>,
    pub a: DateTime<Utc>,
}
