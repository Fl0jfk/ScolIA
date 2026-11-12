//! Stages (socle 39, lots ST0 et ST1 (a)) : dispositifs, paramètres, organismes, sites, contacts,
//! périodes (`docs/socles/39_STAGES.md` §9, §10, §19). Routes `ent-api/src/stages.rs` (préfixe
//! `/stages`) et routes historiques `/entreprises`, `/conventions-stage`. Aucune logique métier.
//!
//! Les états restent des chaînes : `declaree` (stage connu **sans convention signée dans ScolIA**),
//! `cloturee`, `annulee` ; organisme `actif`, `a_verifier`, `ferme`, `deconseille`, `interdit` ;
//! période `prevue`, `publiee`, `close`. Toute modification renvoie la `version` lue (verrou
//! optimiste) ; un refus porte un message en français, sans donnée personnelle.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Dispositif fourni par ScolIA (référence globale) avec son activation dans l'établissement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DispositifStageDto {
    pub code: String,
    pub libelle: String,
    pub observation_seule: bool,
    pub regime_at: bool,
    pub allocation: bool,
    pub actif: bool,
}

/// `PUT /stages/dispositifs/{code}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiverDispositifBody {
    pub actif: bool,
}

/// Paramètres de l'établissement (§19.1). `version` = 0 tant que rien n'a été enregistré.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParametresStageDto {
    pub assureur_nom: Option<String>,
    pub assurance_contrat: Option<String>,
    pub deliberation_ref: Option<String>,
    pub deliberation_le: Option<NaiveDate>,
    pub max_stagiaires_referent: Option<i16>,
    pub max_stagiaires_organisme: Option<i16>,
    pub relance_ouverture_j: i16,
    pub relance_sans_lieu_j: i16,
    pub alerte_veille_heure: NaiveTime,
    pub conservation_contacts_annees: i16,
    pub etablissement_signe_en_dernier: bool,
    pub accord_titulaires: String,
    pub stage_famille: String,
    /// `true` : l'assurance est renseignée (obligatoire avant la première convention, DN-18).
    pub assurance_renseignee: bool,
    pub version: i32,
}

/// `PUT /stages/parametres`. Champ absent = inchangé ; pour les champs facultatifs, `null`
/// explicite = effacer. `version` : celle lue (omise à la première écriture).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModifierParametresStageBody {
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default, deserialize_with = "double_option")]
    pub assurance: Option<Option<AssuranceBody>>,
    #[serde(default, deserialize_with = "double_option")]
    pub deliberation: Option<Option<DeliberationBody>>,
    #[serde(default, deserialize_with = "double_option")]
    pub max_stagiaires_referent: Option<Option<i16>>,
    #[serde(default, deserialize_with = "double_option")]
    pub max_stagiaires_organisme: Option<Option<i16>>,
    #[serde(default)]
    pub relance_ouverture_j: Option<i16>,
    #[serde(default)]
    pub relance_sans_lieu_j: Option<i16>,
    #[serde(default)]
    pub conservation_contacts_annees: Option<i16>,
    #[serde(default)]
    pub etablissement_signe_en_dernier: Option<bool>,
    #[serde(default)]
    pub accord_titulaires: Option<String>,
    #[serde(default)]
    pub stage_famille: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssuranceBody {
    pub assureur_nom: String,
    pub contrat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeliberationBody {
    pub reference: String,
    pub date: NaiveDate,
}

/// Distingue « champ absent » (`None`) de « `null` explicite » (`Some(None)`).
fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// Organisme du carnet (détail). Les contacts ont leur propre lecture : ils ne sont jamais joints
/// à cette réponse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganismeStageDto {
    pub id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
    #[serde(rename = "type")]
    pub type_organisme: String,
    pub enseigne: Option<String>,
    pub code_postal: Option<String>,
    pub commune: Option<String>,
    pub pays: String,
    pub etat: String,
    pub etat_motif_code: Option<String>,
    /// Texte libre court : renvoyé aux seuls porteurs de `stages.organismes.gerer`.
    pub etat_motif_texte: Option<String>,
    pub version: i32,
}

/// `PUT /stages/organismes/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModifierOrganismeBody {
    pub version: i32,
    #[serde(default, rename = "type")]
    pub type_organisme: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub enseigne: Option<Option<String>>,
    #[serde(default)]
    pub adresse: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub code_postal: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub commune: Option<Option<String>>,
    #[serde(default)]
    pub pays: Option<String>,
}

/// `POST /stages/organismes/{id}/etat` (STG-117).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChangerEtatOrganismeBody {
    pub version: i32,
    pub etat: String,
    #[serde(default)]
    pub motif_code: Option<String>,
    #[serde(default)]
    pub motif_texte: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteStageDto {
    pub id: Uuid,
    pub entreprise_id: Uuid,
    pub libelle: String,
    pub adresse_ligne1: String,
    pub adresse_ligne2: Option<String>,
    pub code_postal: String,
    pub commune: String,
    pub pays: String,
    pub actif: bool,
    pub version: i32,
}

/// `POST /stages/organismes/{id}/sites` et `PUT /stages/sites/{id}` (`version` requise en modification).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteStageBody {
    #[serde(default)]
    pub version: Option<i32>,
    pub libelle: String,
    pub adresse_ligne1: String,
    #[serde(default)]
    pub adresse_ligne2: Option<String>,
    pub code_postal: String,
    pub commune: String,
    #[serde(default)]
    pub pays: Option<String>,
}

/// Contact de l'organisme : donnée personnelle d'une personne extérieure, lisible des seuls
/// porteurs de `stages.organismes.gerer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactStageDto {
    pub id: Uuid,
    pub entreprise_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub fonction: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub est_tuteur: bool,
    pub est_signataire: bool,
    pub est_rh: bool,
    pub informe_le: Option<NaiveDate>,
    pub actif: bool,
    pub version: i32,
}

/// `POST /stages/organismes/{id}/contacts` et `PUT /stages/contacts/{id}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactStageBody {
    #[serde(default)]
    pub version: Option<i32>,
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub fonction: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub telephone: Option<String>,
    #[serde(default)]
    pub est_tuteur: bool,
    #[serde(default)]
    pub est_signataire: bool,
    #[serde(default)]
    pub est_rh: bool,
}

/// `POST /stages/contacts/{id}/desactiver`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionBody {
    pub version: i32,
}

/// Public d'une période : exactement un des trois identifiants.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PublicPeriodeDto {
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeriodeStageDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub dispositif_code: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// `collective` ou `individuelle`.
    #[serde(rename = "type")]
    pub type_periode: String,
    pub motif: Option<String>,
    pub etat: String,
    pub publics: Vec<PublicPeriodeDto>,
    pub version: i32,
}

/// `POST /stages/periodes`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreerPeriodeStageBody {
    pub annee_id: Uuid,
    pub dispositif_code: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    #[serde(default)]
    pub individuelle: bool,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub publics: Vec<PublicPeriodeDto>,
}

/// `PUT /stages/periodes/{id}` : dates, libellé, motif et publics (remplacés).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModifierPeriodeStageBody {
    pub version: i32,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    #[serde(default)]
    pub motif: Option<String>,
    pub publics: Vec<PublicPeriodeDto>,
}

/// Filtre de `GET /stages/periodes`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FiltrePeriodesStage {
    #[serde(default)]
    pub annee_id: Option<Uuid>,
    #[serde(default)]
    pub etat: Option<String>,
}
