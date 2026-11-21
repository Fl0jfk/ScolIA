//! Personnels (socle 6, lot P3) : services d'enseignement et interventions
//! (`docs/socles/06_PERSONNELS.md` §8.5, §10.4, PRS-40 à PRS-49 et PRS-54).
//! Routes `ent-api/src/personnels_services.rs` (préfixe `/personnels/services`). Aucune logique métier.
//!
//! Un **service** dit « qui assure quel cours » (le cours, `ent_cours`, est l'objet du socle 5) ; une
//! **intervention** est la présence datée d'un agent sur un service. Les codes (modalité, état,
//! nature) restent des chaînes : la liste fermée vit dans `ent-db` et dans la base.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Titulaire posé à la création d'un service (facultatif : un service peut naître vacant).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TitulaireInitialBody {
    pub agent_id: Uuid,
    /// Défaut : début du service, sinon début de l'année.
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub heures_hebdo: Option<f64>,
}

/// `POST /personnels/services` (PRS-40). Soit `cours_id` (cours existant du socle 5), soit une cible
/// (`annee_id` + `classe_id` ou `groupe_id`) et une matière : le cours ouvert est trouvé ou créé.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CreerServiceBody {
    #[serde(default)]
    pub cours_id: Option<Uuid>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_libelle: Option<String>,
    /// `cours` (défaut), `tp`, `co_intervention`, `accompagnement`, `option`, `dispositif`, `autre`.
    #[serde(default)]
    pub modalite: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub heures_hebdo: Option<f64>,
    #[serde(default)]
    pub heures_annuelles: Option<f64>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// `ouvert` (défaut) ou `prevu` (répartition de l'année suivante : aucun effet sur les droits).
    #[serde(default)]
    pub etat: Option<String>,
    #[serde(default)]
    pub titulaire: Option<TitulaireInitialBody>,
}

/// `PUT /personnels/services/{id}` (PRS-41) : seuls les champs présents changent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModifierServiceBody {
    #[serde(default)]
    pub modalite: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub heures_hebdo: Option<f64>,
    #[serde(default)]
    pub heures_annuelles: Option<f64>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// Efface la date de début (retour aux bornes de l'année).
    #[serde(default)]
    pub effacer_du: bool,
    #[serde(default)]
    pub effacer_au: bool,
    #[serde(default)]
    pub effacer_code: bool,
}

/// `POST /personnels/services/{id}/remplacer` (PRS-42) : nouvelle cible ou nouvelle matière à une
/// date ; l'ancien service est clos la veille, ses évaluations et son cahier ne sont JAMAIS
/// rattachés automatiquement (la réponse en compte les éléments à rattacher).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemplacerServiceBody {
    pub a_partir_du: NaiveDate,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_libelle: Option<String>,
    /// Reporte les interventions actives sur le nouveau service (défaut : oui).
    #[serde(default = "vrai")]
    pub reprendre_intervenants: bool,
}

fn vrai() -> bool {
    true
}

impl Default for RemplacerServiceBody {
    fn default() -> Self {
        Self {
            a_partir_du: NaiveDate::default(),
            classe_id: None,
            groupe_id: None,
            matiere_id: None,
            matiere_libelle: None,
            reprendre_intervenants: true,
        }
    }
}

/// `POST /personnels/services/{id}/clore` (PRS-43).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloreServiceBody {
    /// Dernier jour du service (défaut : aujourd'hui).
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

/// `POST /personnels/services/{id}/intervenants` (PRS-44, PRS-45).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AjouterIntervenantBody {
    pub agent_id: Uuid,
    /// `titulaire`, `co_enseignant`, `assistant`, `intervenant`, `stagiaire_observateur`.
    /// Le suppléant se nomme par `nommer-suppleant`.
    pub nature: String,
    /// Défaut : début du service (titulaire) ou aujourd'hui.
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub heures_hebdo: Option<f64>,
    /// Restriction facultative : `appel`, `notes`, `cahier`, `appreciations`.
    #[serde(default)]
    pub droits: Option<Vec<String>>,
}

/// `POST /personnels/services/{id}/suppleance` (PRS-46).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NommerSuppleantBody {
    pub agent_id: Uuid,
    pub du: NaiveDate,
    /// Absent : suppléance ouverte (jusqu'au retour du titulaire).
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// `absence` ou `vacance` ; défaut `vacance` si le service est sans titulaire, sinon `absence`.
    #[serde(default)]
    pub origine: Option<String>,
    /// Absence du personnel (socle 15), sans clé étrangère.
    #[serde(default)]
    pub origine_ref: Option<Uuid>,
}

/// `POST /personnels/interventions/{id}/terminer`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TerminerInterventionBody {
    /// Dernier jour de l'intervention (défaut : aujourd'hui).
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

/// Intervention de remplacement dans une correction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InterventionCorrigeeBody {
    pub agent_id: Uuid,
    pub nature: String,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub heures_hebdo: Option<f64>,
}

/// `POST /personnels/interventions/{id}/corriger` (PRS-47) : l'erreur est annulée (jamais effacée),
/// éventuellement remplacée par une ligne exacte. Les saisies déjà faites gardent leur auteur réel.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CorrigerInterventionBody {
    /// `erreur_saisie`, `doublon` ou `autre`.
    pub motif: String,
    #[serde(default)]
    pub remplacement: Option<InterventionCorrigeeBody>,
}

/// `POST /personnels/services/remplacer-agent` (PRS-48) : un successeur reprend les interventions
/// d'un agent. `apercu = true` ne change rien et liste ce qui serait fait.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RemplacerAgentBody {
    pub agent_sortant_id: Uuid,
    pub agent_entrant_id: Uuid,
    pub a_partir_du: NaiveDate,
    /// Limite à ces services (défaut : tous ceux où le sortant intervient encore).
    #[serde(default)]
    pub service_ids: Option<Vec<Uuid>>,
    #[serde(default)]
    pub apercu: bool,
}

/// Alerte de cohérence : jamais bloquante (§8.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteServiceDto {
    /// `hors_engagement`, `heures_depassees`, `doublon_probable`, `vacant`.
    pub code: String,
    #[serde(default)]
    pub service_id: Option<Uuid>,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterventionDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    /// Désignation de l'agent (nom d'usage), pour l'affichage interne.
    pub agent: String,
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub heures_hebdo: Option<f64>,
    /// Saisie prévue par la nature.
    pub saisie: bool,
    /// Saisie effective à la date de la requête (un suppléant retire la saisie au titulaire).
    pub saisie_effective: bool,
    pub droits: Option<Vec<String>>,
    pub origine: String,
    pub origine_ref: Option<Uuid>,
    pub annulee: bool,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub modalite: String,
    pub code: Option<String>,
    pub heures_hebdo: Option<f64>,
    pub heures_annuelles: Option<f64>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub etat: String,
    pub source: String,
    pub remplace_service_id: Option<Uuid>,
    /// Aucune intervention en saisie à la date de la requête (service ouvert seulement).
    pub vacant: bool,
    pub intervenants: Vec<InterventionDto>,
    pub alertes: Vec<AlerteServiceDto>,
}

/// Filtres de `GET /personnels/services`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FiltreServices {
    #[serde(default)]
    pub annee_id: Option<Uuid>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    #[serde(default)]
    pub etat: Option<String>,
    #[serde(default)]
    pub vacants: Option<bool>,
    /// Date de calcul (défaut : aujourd'hui, fuseau de l'établissement).
    #[serde(default)]
    pub le: Option<NaiveDate>,
}

/// Réponse de la répartition (PRS-49) : services, couverture par agent, alertes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepartitionDto {
    pub le: NaiveDate,
    pub services: Vec<ServiceDto>,
    pub agents: Vec<CouvertureAgentDto>,
    pub vacants: i64,
}

/// Heures attribuées contre heures dues d'un agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CouvertureAgentDto {
    pub agent_id: Uuid,
    pub agent: String,
    pub heures_attribuees: f64,
    pub heures_dues: Option<f64>,
    pub services: i64,
}

/// `GET /personnels/moi/services` (PRS-54) : les services de l'agent connecté.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MesServicesDto {
    pub le: NaiveDate,
    pub agent_id: Option<Uuid>,
    pub services: Vec<ServiceDto>,
    pub heures_attribuees: f64,
    pub heures_dues: Option<f64>,
    pub alertes: Vec<AlerteServiceDto>,
}

/// Une ligne de l'aperçu ou du résultat d'un remplacement d'agent (PRS-48).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LigneRemplacementDto {
    pub service_id: Uuid,
    pub intervention_id: Uuid,
    pub nature: String,
    /// `a_faire` (aperçu), `fait`, ou une raison de non-application (`deja_intervenant`, `deja_termine`).
    pub resultat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemplacementAgentDto {
    pub apercu: bool,
    pub lignes: Vec<LigneRemplacementDto>,
}

/// Résultat d'un remplacement de service (PRS-42).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceRemplaceDto {
    pub ancien_id: Uuid,
    pub nouveau: ServiceDto,
    /// Devoirs rattachés à l'ancien service : à rattacher au nouveau PAR UNE DÉCISION, jamais d'office.
    pub devoirs_a_rattacher: i64,
}

/// Créneau d'emploi du temps sans service (indicateur de couverture).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreneauSansServiceDto {
    pub creneau_id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub jour_semaine: i16,
    pub heure_debut: String,
    pub heure_fin: String,
    pub agent_id: Option<Uuid>,
}

/// Intervenant tel que rendu aux autres socles (ports `intervenants`, `en_saisie`, `de_l_eleve`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntervenantPortDto {
    pub service_id: Uuid,
    pub intervention_id: Uuid,
    pub agent_id: Uuid,
    /// Compte de l'agent s'il en a un.
    pub user_id: Option<Uuid>,
    pub nature: String,
    pub saisie_effective: bool,
    pub droits: Option<Vec<String>>,
}
