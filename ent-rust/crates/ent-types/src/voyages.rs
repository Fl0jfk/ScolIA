//! Types JSON des routes `/voyages/*` (socle 42, lot V0a : sorties et voyages scolaires).
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
