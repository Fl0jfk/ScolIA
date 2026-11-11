//! Types JSON des routes `/internat/*` (socle 41, lot I1 : hébergement, séjours, rythmes,
//! affectations sous contraintes, dérogations, plan, paramètres).
//!
//! Aucune règle ici : le serveur décide (droits, régime daté, public de la chambre, interdiction de
//! contact, capacité). Les états restent des chaînes (`"active"`, `"filles"`…). **Aucune donnée de
//! santé** (R-IT6) : ni allergie, ni traitement, ni diagnostic dans ces types. Le plan ne porte que
//! des identifiants d'élèves, jamais de nom (R-IT32).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────── Zones ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoneDto {
    pub id: Uuid,
    pub batiment_id: Uuid,
    pub code: String,
    pub libelle: String,
    /// `filles`, `garcons` ou `commun` (parties communes : aucune chambre).
    pub public: String,
    pub ordre: i16,
    pub ferme_du: Option<NaiveDate>,
    pub ferme_au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoneSaisieDto {
    pub batiment_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub public: String,
    #[serde(default)]
    pub ordre: Option<i16>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneModifDto {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub public: Option<String>,
    #[serde(default)]
    pub ordre: Option<i16>,
    /// Fermeture (travaux) : `ferme_du` obligatoire, `ferme_au` absent = jusqu'à nouvel ordre.
    #[serde(default)]
    pub ferme_du: Option<NaiveDate>,
    #[serde(default)]
    pub ferme_au: Option<NaiveDate>,
    /// Rouvrir la zone (efface la fermeture).
    #[serde(default)]
    pub rouvrir: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoneModifieeDto {
    pub zone: ZoneDto,
    /// Occupants (affectations ouvertes) des chambres de la zone, à reloger si elle ferme (INT-101).
    pub occupants_a_reloger: i64,
}

// ───────────────────────── Chambres et lits ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChambreSaisieDto {
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    pub batiment: String,
    #[serde(default)]
    pub etage: i16,
    pub code: String,
    /// Nombre de lits, de 1 à 8.
    pub capacite: i16,
    /// `filles` ou `garcons` (aucune chambre mixte, R-IT11).
    pub public: String,
    #[serde(default)]
    pub accessible_pmr: bool,
    #[serde(default)]
    pub douche: bool,
    #[serde(default)]
    pub lavabo: bool,
    #[serde(default)]
    pub bureau: bool,
    /// Sans nom d'élève (R-IT32).
    #[serde(default)]
    pub notes_entretien: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChambreModifDto {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub capacite: Option<i16>,
    #[serde(default)]
    pub public: Option<String>,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub retirer_zone: bool,
    #[serde(default)]
    pub accessible_pmr: Option<bool>,
    #[serde(default)]
    pub douche: Option<bool>,
    #[serde(default)]
    pub lavabo: Option<bool>,
    #[serde(default)]
    pub bureau: Option<bool>,
    /// `active`, `fermee` ou `retiree`.
    #[serde(default)]
    pub etat: Option<String>,
    #[serde(default)]
    pub ferme_du: Option<NaiveDate>,
    #[serde(default)]
    pub ferme_au: Option<NaiveDate>,
    #[serde(default)]
    pub effacer_fermeture: bool,
    #[serde(default)]
    pub notes_entretien: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChambreEtendueDto {
    pub id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub public: Option<String>,
    pub zone_id: Option<Uuid>,
    pub accessible_pmr: bool,
    pub douche: bool,
    pub lavabo: bool,
    pub bureau: bool,
    pub etat: String,
    pub ferme_du: Option<NaiveDate>,
    pub ferme_au: Option<NaiveDate>,
    pub notes_entretien: Option<String>,
    pub actif: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LitDto {
    pub id: Uuid,
    pub chambre_id: Uuid,
    pub numero: i16,
    pub neutralise_du: Option<NaiveDate>,
    pub neutralise_au: Option<NaiveDate>,
    pub motif_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeutraliserLitDto {
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// `casse`, `accueil_urgence`, `travaux` ou `autre`.
    pub motif_code: String,
}

// ───────────────────────── Séjours et rythmes ─────────────────────────

/// Soirs attendus (une case par jour de la semaine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoursRythmeDto {
    pub dimanche: bool,
    pub lundi: bool,
    pub mardi: bool,
    pub mercredi: bool,
    pub jeudi: bool,
    pub vendredi: bool,
    pub samedi: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RythmeDto {
    /// La version vaut à partir de cette date (R-IT4).
    pub du: NaiveDate,
    #[serde(flatten)]
    pub jours: JoursRythmeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RythmeSaisieDto {
    pub du: NaiveDate,
    #[serde(flatten)]
    pub jours: JoursRythmeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SejourDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// `regime` (naissance normale, R-IT1), `reprise` ou `manuel_direction`.
    pub origine: String,
    pub ferme_motif_code: Option<String>,
    /// Versions du rythme, la plus récente en dernier.
    pub rythmes: Vec<RythmeDto>,
}

// ───────────────────────── Affectations ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AffectationSaisieDto {
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    /// Lit choisi ; absent = premier lit libre de la chambre.
    #[serde(default)]
    pub lit_id: Option<Uuid>,
    pub debut: NaiveDate,
    /// `rentree`, `demande_eleve`, `conflit`, `travaux`, `sante` (affiché « autre »), `relogement`, `autre`.
    #[serde(default)]
    pub motif_code: Option<String>,
    /// L'élève a un besoin d'accessibilité : seule une chambre accessible convient.
    #[serde(default)]
    pub besoin_accessibilite: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChangementChambreDto {
    pub chambre_id: Uuid,
    #[serde(default)]
    pub lit_id: Option<Uuid>,
    /// Premier jour dans la nouvelle chambre ; l'ancienne affectation prend fin la veille.
    pub date_effet: NaiveDate,
    pub motif_code: String,
    #[serde(default)]
    pub besoin_accessibilite: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AffectationHebergementDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit_id: Option<Uuid>,
    pub lit_numero: Option<i16>,
    pub sejour_id: Option<Uuid>,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub motif_code: Option<String>,
}

// ───────────────────────── Dérogations et interdictions ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DerogationSaisieDto {
    pub eleve_id: Uuid,
    /// `public_chambre` ou `interdiction_contact`.
    pub nature: String,
    #[serde(default)]
    pub chambre_id: Option<Uuid>,
    #[serde(default)]
    pub avec_eleve_id: Option<Uuid>,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DerogationDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub chambre_id: Option<Uuid>,
    pub avec_eleve_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub decide_par_user_id: Uuid,
    pub cree_le: DateTime<Utc>,
}

/// Port provisoire de la mesure « interdiction de contact » du socle 13 (jamais de motif ici).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterdictionContactSaisieDto {
    pub eleve_a_id: Uuid,
    pub eleve_b_id: Uuid,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterdictionContactDto {
    pub id: Uuid,
    pub eleve_a_id: Uuid,
    pub eleve_b_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

// ───────────────────────── Plan des lits (INT-107) ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanLitDto {
    pub lit_id: Uuid,
    pub numero: i16,
    /// `libre`, `occupe` ou `neutralise`.
    pub etat: String,
    /// Identifiant seulement : le plan d'un écran commun ne porte aucun nom (R-IT32).
    pub eleve_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanChambreDto {
    pub id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub public: Option<String>,
    pub accessible_pmr: bool,
    pub etat: String,
    pub capacite: i16,
    pub lits: Vec<PlanLitDto>,
    /// Occupants hérités sans lit attribué (affectations antérieures à la migration 2823).
    pub occupants_sans_lit: Vec<Uuid>,
    /// Plus d'occupants que de lits.
    pub surcapacite: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanZoneDto {
    pub zone: ZoneDto,
    pub chambres: Vec<PlanChambreDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanDto {
    pub jour: NaiveDate,
    pub zones: Vec<PlanZoneDto>,
    pub chambres_sans_zone: Vec<PlanChambreDto>,
    pub lits_libres: i64,
    pub lits_occupes: i64,
    pub lits_neutralises: i64,
}

// ───────────────────────── Paramètres (S-4100 à S-4117) ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParametresInternatDto {
    pub heure_limite_annonce: NaiveTime,
    pub heure_limite_appel: NaiveTime,
    pub seuil_retard_minutes: i16,
    pub heure_recuperation_vendredi: NaiveTime,
    /// 1 = lundi … 7 = dimanche.
    pub jour_bilan: i16,
    pub voisins_visibles_familles: bool,
    pub depot_telephones: bool,
    pub garde_honorabilite: bool,
    pub impression_presents: bool,
    /// `toujours`, `si_a_charge` ou `jamais`.
    pub majeur_info_parents: String,
    /// Rythme par défaut d'un nouveau séjour.
    pub rythme_defaut: JoursRythmeDto,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ParametresInternatModifDto {
    #[serde(default)]
    pub heure_limite_annonce: Option<NaiveTime>,
    #[serde(default)]
    pub heure_limite_appel: Option<NaiveTime>,
    #[serde(default)]
    pub seuil_retard_minutes: Option<i16>,
    #[serde(default)]
    pub heure_recuperation_vendredi: Option<NaiveTime>,
    #[serde(default)]
    pub jour_bilan: Option<i16>,
    #[serde(default)]
    pub voisins_visibles_familles: Option<bool>,
    #[serde(default)]
    pub depot_telephones: Option<bool>,
    #[serde(default)]
    pub garde_honorabilite: Option<bool>,
    #[serde(default)]
    pub impression_presents: Option<bool>,
    #[serde(default)]
    pub majeur_info_parents: Option<String>,
    #[serde(default)]
    pub rythme_defaut: Option<JoursRythmeDto>,
}

/// Fait de présence d'une nuit (historique, rien n'est effacé : R-IT13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresenceFaitDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nuit: NaiveDate,
    pub marque: String,
    pub note: String,
    pub motif_correction_code: Option<String>,
    pub par_user_id: Uuid,
    pub recu_le: DateTime<Utc>,
}
