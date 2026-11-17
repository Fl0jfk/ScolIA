//! Pastilles (indicateurs) et synthèse de la fiche (socle 2, lot E8). Routes `ent-api` sous
//! `/eleves/{id}/synthese`, `/eleves/{id}/pastilles`, `/eleves/pastilles…`. Aucune logique métier.
//!
//! Le **contrat des tuiles** : chaque domaine déclare son emplacement dans la synthèse ; une tuile
//! sans droit n'est pas vide, elle est `masquee` avec sa `raison` et ne laisse rien filtrer.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Une pastille visible de l'acteur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PastilleDto {
    pub code: String,
    /// Domaine propriétaire (code de tuile).
    pub domaine: String,
    pub libelle: String,
    pub icone: String,
    /// `alerte`, `attention`, `information`.
    pub gravite: String,
    /// Valeur courte (régime, type de mobilité…), quand la pastille en porte une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur_code: Option<String>,
    /// Échéance (fin d'un PAI, retour de mobilité, date de majorité…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur_date: Option<NaiveDate>,
    pub du: NaiveDate,
}

/// Une tuile de la grille « Synthèse ».
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TuileDto {
    pub domaine: String,
    pub libelle: String,
    pub rang: u8,
    pub lien: String,
    /// `visible` ou `masquee`.
    pub etat: String,
    /// Présente seulement si la tuile est masquée : `droit_manquant`, `hors_portee`,
    /// `reserve_au_personnel`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raison: Option<String>,
    /// Vide si la tuile est masquée.
    pub pastilles: Vec<PastilleDto>,
}

/// `GET /eleves/{id}/synthese`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyntheseDto {
    pub eleve_id: Uuid,
    /// Alerte d'abord.
    pub pastilles: Vec<PastilleDto>,
    /// Dans l'ordre du contrat (`rang`).
    pub tuiles: Vec<TuileDto>,
}

/// `GET /eleves/pastilles` : combien d'élèves portent la pastille (à l'échelle de l'établissement).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompteurPastilleDto {
    pub code: String,
    pub libelle: String,
    pub gravite: String,
    pub nombre: i64,
}

/// `GET /eleves/pastilles/{code}` : la liste de travail.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevesPastilleDto {
    pub code: String,
    pub eleves: Vec<Uuid>,
}

/// Une pastille du catalogue du logiciel (`GET /eleves/pastilles/catalogue`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndicateurCatalogueDto {
    pub code: String,
    pub domaine: String,
    pub libelle: String,
    pub icone: String,
    pub gravite: String,
}

/// Le contrat d'une tuile (`GET /eleves/pastilles/catalogue`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContratTuileDto {
    pub domaine: String,
    pub libelle: String,
    pub rang: u8,
    pub personnel_seul: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogueDto {
    pub indicateurs: Vec<IndicateurCatalogueDto>,
    pub tuiles: Vec<ContratTuileDto>,
}
