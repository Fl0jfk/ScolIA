//! Pastilles (indicateurs) et synthèse de la fiche (socle 2, lot E8). Routes `ent-api` sous
//! `/eleves/{id}/synthese`, `/eleves/{id}/pastilles`, `/eleves/pastilles…`. Aucune logique métier.
//!
//! Le **contrat des tuiles** : chaque domaine déclare son emplacement dans la synthèse ; une tuile
//! sans droit n'est pas vide, elle est `masquee` avec sa `raison` et ne laisse rien filtrer.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::dossier::MasqueChampsDto;
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
    /// Échéance (fin d'un PAI, retour de mobilité…). **Jamais** pour les pastilles qui ne livrent que le
    /// fait (vie de classe, majorité, protection).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur_date: Option<NaiveDate>,
    /// Depuis quand ; absent pour les pastilles qui ne livrent que le fait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub du: Option<NaiveDate>,
}

/// Une tuile de la grille « Synthèse ».
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TuileDto {
    pub domaine: String,
    pub libelle: String,
    pub rang: u8,
    /// Absent quand la tuile est masquée (aucun chemin vers un écran fermé).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lien: Option<String>,
    /// `visible` ou `masquee`.
    pub etat: String,
    /// Présente seulement si la tuile est masquée : toujours `droit_manquant` (le serveur ne détaille pas
    /// pourquoi, pour ne rien laisser déduire des droits d'autrui). Les autres valeurs restent lisibles
    /// par compatibilité.
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

/// Une ligne de la liste des élèves (`GET /eleves/liste`) : désignation minimale. Ni photo, ni
/// identifiant, ni date : ce que le lecteur n'a pas le droit de voir n'est pas dans la ligne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveLigneDto {
    pub id: Uuid,
    pub prenom: String,
    pub nom: String,
    #[serde(default)]
    pub classe: Option<String>,
    /// `actif`, `preinscrit` ou `sorti`.
    pub statut: String,
}

/// `GET /eleves/liste?q=&classe=&statut=&pastille=&limite=&decalage=` : une page de la liste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveListeDto {
    pub eleves: Vec<EleveLigneDto>,
    /// Nombre d'élèves qui répondent aux filtres (pas seulement ceux de la page).
    pub total: i64,
    pub decalage: i64,
    pub limite: i64,
    /// La recherche a atteint son plafond : affiner pour ne rien manquer.
    pub tronque: bool,
    /// `etablissement` (tout l'annuaire) ou `perimetre` (seulement les élèves de la portée du lecteur,
    /// par exemple les classes d'un professeur).
    pub portee: String,
    /// Classes présentes dans les résultats (avant le filtre de classe), pour le sélecteur.
    pub classes: Vec<String>,
}

/// `GET /eleves/{id}` : l'élève tel que CE lecteur peut le voir (minimisé par groupes de champs, E1).
/// Même JSON que l'`EleveDto` du serveur (un test d'`ent-api` garde l'identité des deux formes) ; un
/// champ absent est un champ que le lecteur n'a pas le droit de voir, expliqué dans `masques`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EleveFicheDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub ine: Option<String>,
    #[serde(default)]
    pub sexe: Option<String>,
    #[serde(default)]
    pub date_naissance: Option<NaiveDate>,
    #[serde(default)]
    pub numero_badge: Option<String>,
    /// `null` quand la scolarité est masquée (donnée non communiquée, pas un faux « non »).
    #[serde(default)]
    pub redoublant: Option<bool>,
    pub status: String,
    #[serde(default)]
    pub classe: Option<String>,
    #[serde(default)]
    pub regime: Option<String>,
    #[serde(default)]
    pub mef: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
    #[serde(default)]
    pub date_entree: Option<NaiveDate>,
    #[serde(default)]
    pub date_sortie: Option<NaiveDate>,
    /// Groupes de champs masqués, avec la raison et le message (jamais de masquage muet).
    #[serde(default)]
    pub masques: Vec<MasqueChampsDto>,
}
