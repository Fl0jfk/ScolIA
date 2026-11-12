//! Administration du référentiel (socle 2, lot S2-2) : contrat JSON des routes
//! `/referentiel/*` (années et bascule, sites, niveaux, MEF, périodes, calendrier, grille,
//! séances, groupes). Le JSON est celui des structures d'`ent-db` (champs inconnus ignorés) ;
//! un test contre le vrai backend vérifie le décodage. Aucune logique métier.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Année scolaire avec son cycle de vie (`preparation` | `courante` | `cloturee`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnneeStatutDto {
    pub id: Uuid,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub courante: bool,
    pub statut: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportReconductionDto {
    pub annee_cible_id: Uuid,
    pub classes_creees: i64,
    pub groupes_crees: i64,
    pub periodes_creees: i64,
    pub services_crees: i64,
}

/// Réponse de `POST /referentiel/annees/{id}/preparer-suivante`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparationDto {
    pub annee: AnneeStatutDto,
    pub rapport: RapportReconductionDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportOuvertureDto {
    pub annee_id: Uuid,
    #[serde(default)]
    pub annee_cloturee_id: Option<Uuid>,
    pub eleves_synchronises: i64,
}

/// Aperçu ou résultat d'une affectation en lot ou d'une montée.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportLotDto {
    pub apercu: bool,
    pub a_creer: i64,
    pub deja_conformes: i64,
    #[serde(default)]
    pub conflits: Vec<Uuid>,
    #[serde(default)]
    pub ignores: Vec<Uuid>,
}

/// Correspondance de montée : classe source → classe cible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrespondanceDto {
    pub source: Uuid,
    pub cible: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub adresse: Option<String>,
    #[serde(default)]
    pub uai: Option<String>,
    pub actif: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NiveauDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub libelle_court: Option<String>,
    pub ordre: i16,
    #[serde(default)]
    pub cycle: Option<i16>,
    pub source: String,
    pub actif: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MefDto {
    pub id: Uuid,
    pub code_mef: String,
    #[serde(default)]
    pub mefstat11: Option<String>,
    #[serde(default)]
    pub niveau_id: Option<Uuid>,
    pub libelle_long: String,
    #[serde(default)]
    pub libelle_edition: Option<String>,
    #[serde(default)]
    pub code_formation_diplome: Option<String>,
    pub source: String,
    pub actif: bool,
}

/// Ligne d'import MEF (corps de `POST /referentiel/mef/import`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MefImportDto {
    pub code_mef: String,
    #[serde(default)]
    pub mefstat11: Option<String>,
    #[serde(default)]
    pub niveau_code: Option<String>,
    pub libelle_long: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportImportMefDto {
    pub crees: i64,
    pub mis_a_jour: i64,
    /// (code MEF, raison).
    #[serde(default)]
    pub rejets: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmorcageNiveauxDto {
    pub niveaux_crees: i64,
    pub classes_rattachees: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodeDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    #[serde(default)]
    pub niveau_id: Option<Uuid>,
    pub code: String,
    pub libelle: String,
    /// `trimestre` | `semestre` | `periode`.
    pub kind: String,
    pub ordre: i16,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

/// Période envoyée à `PUT /referentiel/annees/{id}/periodes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodeSaisieDto {
    pub code: String,
    pub libelle: String,
    pub kind: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

/// Jour neutralisé du calendrier (vacances, férié, fermeture, journée banalisée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourCalendrierDto {
    pub jour: NaiveDate,
    pub kind: String,
    pub libelle: String,
    #[serde(default)]
    pub site_id: Option<Uuid>,
    /// `jour` | `vacances`.
    pub origine: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauGrilleDto {
    pub id: Uuid,
    #[serde(default)]
    pub site_id: Option<Uuid>,
    pub code: String,
    pub libelle: String,
    /// `matin` | `apres_midi`.
    pub demi_journee: String,
    pub ordre: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauGrilleSaisieDto {
    pub code: String,
    pub libelle: String,
    pub demi_journee: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportSeancesDto {
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    pub creees: i64,
    pub mises_a_jour: i64,
    pub supprimees: i64,
    /// Séances porteuses (cahier, travail) retirées au lieu d'être supprimées (socle 10, D1).
    #[serde(default)]
    pub retirees: i64,
    pub jours_neutralises: i64,
    pub creneaux_ab_ignores: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceDto {
    pub id: Uuid,
    pub jour: NaiveDate,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub matiere_libelle: String,
    #[serde(default)]
    pub salle: Option<String>,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub grille_code: Option<String>,
    pub statut: String,
}

/// Responsable d'un groupe ou atelier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableGroupeDto {
    pub user_id: Uuid,
    pub label: String,
}

/// Groupe ou atelier (`GET /referentiel/groupes-admin`) ; `classe_id` nul = atelier transverse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeAdminDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub responsables: Vec<ResponsableGroupeDto>,
}

/// Personnel trouvé par `GET /personnel/search` (choix d'un responsable).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonnelDto {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub label: String,
}
