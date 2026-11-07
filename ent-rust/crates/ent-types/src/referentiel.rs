//! Référentiel scolaire : années, classes, groupes, affectations, services d'enseignement
//! (contrat des routes `/referentiel/*`). Types nouveaux, partagés par `ent-api` et le
//! frontend ; aucune logique métier.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Année scolaire d'un établissement (`libelle` : « 2026-2027 »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnneeScolaireDto {
    pub id: Uuid,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub courante: bool,
}

/// Corps de `POST /referentiel/annees`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerAnneeBody {
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    #[serde(default)]
    pub courante: bool,
}

/// Classe avec son effectif à la date demandée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClasseDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub niveau: Option<String>,
    pub effectif: i64,
    #[serde(default)]
    pub professeur_principal_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professeur_principal_label: Option<String>,
}

/// Corps de `POST /referentiel/classes` (année courante si `annee_id` est absent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerClasseBody {
    pub code: String,
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub niveau: Option<String>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
}

/// Groupe (demi-groupe d'une classe, ou groupe transverse si `classe_id` est nul).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
}

/// Corps de `POST /referentiel/groupes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerGroupeBody {
    pub code: String,
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
}

/// Élève affecté à une classe (identité minimisée selon le rôle de l'appelant).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveClasseDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub ine: Option<String>,
    #[serde(default)]
    pub date_naissance: Option<NaiveDate>,
    pub status: String,
}

/// Service d'enseignement : un professeur, une matière, une classe ou un groupe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub prof_user_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prof_label: Option<String>,
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    pub professeur_principal: bool,
}

/// Corps de `POST /referentiel/services` (classe OU groupe, jamais les deux).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerServiceBody {
    pub prof_user_id: Uuid,
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub professeur_principal: bool,
}

/// Détail de `GET /referentiel/classes/{id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClasseDetailDto {
    pub classe: ClasseDto,
    pub jour: NaiveDate,
    pub eleves: Vec<EleveClasseDto>,
    pub services: Vec<ServiceDto>,
    pub groupes: Vec<GroupeDto>,
}

/// Corps de `POST /referentiel/classes/{id}/affectations`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffecterEleveBody {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub date_debut: Option<NaiveDate>,
}

/// Affectation enregistrée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectationDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    pub date_debut: NaiveDate,
    #[serde(default)]
    pub date_fin: Option<NaiveDate>,
}

/// Résultat de `POST /referentiel/amorcer` (idempotent : un second appel crée 0).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportAmorcageDto {
    pub annee_id: Uuid,
    pub annee_libelle: String,
    pub classes_creees: i64,
    pub affectations_creees: i64,
    pub services_crees: i64,
    /// Élèves dont la classe saisie (texte) diffère de l'affectation déjà enregistrée.
    pub divergences: i64,
}

/// Un enseignement du professeur : une classe entière ou un groupe, avec sa matière et
/// son effectif (`GET /referentiel/mes-enseignements`). Liste unique : l'écran n'a pas à
/// distinguer classe et groupe. L'effectif ne compte que les élèves que l'acteur est
/// autorisé à suivre (même règle que `eleves_autorises`, D-022).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnseignementDto {
    /// Service d'enseignement (clé des grilles de notes et du cahier de textes).
    pub service_id: Uuid,
    /// `classe` (classe entière) ou `groupe`.
    pub nature: String,
    /// Classe entière, ou classe parente du groupe si elle existe.
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    /// Code de la classe ou du groupe (« 4B », « 4B-ALL »).
    pub cible_code: String,
    /// Code de la classe (celui du groupe pour un atelier transverse : vide).
    pub classe_code: String,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub effectif: i64,
    pub professeur_principal: bool,
    /// Libellé prêt à afficher : « Allemand · 4B-ALL (groupe) ».
    pub libelle: String,
}
