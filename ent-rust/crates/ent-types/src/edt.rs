//! Emploi du temps (contrat de `GET /edt/mes-creneaux`). Copie fidèle de `ent-api/src/dto.rs`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Créneau d'emploi du temps. `jour_semaine` : 1 = lundi … 7 = dimanche (ISO).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdtCreneauDto {
    pub id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prof_label: Option<String>,
    pub jour_semaine: i16,
    pub heure_debut: String,
    pub heure_fin: String,
    pub salle: Option<String>,
    pub semaine_ab: String,
}
