//! Appel (contrat de `POST /appels`, `GET /appels/{id}`, `POST /appels/{id}/lignes`,
//! `POST /appels/{id}/clore`). Copie fidèle de `ent-api/src/dto.rs`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Corps de `POST /appels`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvrirAppelBody {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
}

/// Corps de `POST /appels/{id}/lignes`. `etat` : `present` | `absent` | `retard`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarquerAppelLigneBody {
    pub eleve_id: Uuid,
    pub etat: String,
}

/// Ligne d'un appel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppelLigneDto {
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub etat: String,
}

/// Appel. `statut` : `ouvert` | `clos`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppelDto {
    pub id: Uuid,
    pub classe: String,
    pub jour: NaiveDate,
    pub statut: String,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
    pub lignes: Vec<AppelLigneDto>,
}

impl AppelDto {
    /// L'appel est clos (plus modifiable).
    pub fn est_clos(&self) -> bool {
        self.statut == "clos"
    }
}
