//! Droits effectifs du moteur D-017 : `GET /roles/mes-droits`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

/// Un droit effectif : action du catalogue (`referentiel.gerer`…) et portée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitEffectif {
    pub action: String,
    pub portee: String,
    #[serde(default)]
    pub cibles: Vec<Uuid>,
}

impl ApiClient {
    /// `GET /roles/mes-droits` : profil de base + rôles paramétrés attribués.
    pub async fn mes_droits(&self) -> Result<Vec<DroitEffectif>, ApiError> {
        self.appel(Methode::Get, "/roles/mes-droits", None).await
    }
}
