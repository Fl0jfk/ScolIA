//! À-faire unifié (socle 29, lots P2 et P3) : routes `/afaire*` d'`ent-api`.
//! Le serveur revérifie chaque tâche à la lecture et chaque droit à l'exécution ; ce client transporte.

use ent_types::afaire::{AFaire, ApercuAction, ExecutionAction, ResultatAction};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /afaire` : l'à-faire de la personne connectée (jamais mis en cache).
    pub async fn a_faire(&self) -> Result<AFaire, ApiError> {
        self.appel(Methode::Get, "/afaire", None).await
    }

    /// `POST /afaire/{id}/apercu` : aperçu de la carte d'action (aucune écriture).
    pub async fn a_faire_apercu(&self, tache: Uuid) -> Result<ApercuAction, ApiError> {
        self.appel(Methode::Post, &format!("/afaire/{tache}/apercu"), None)
            .await
    }

    /// `POST /afaire/{id}/executer` : exécute la commande du propriétaire après confirmation.
    /// Une démarche déjà traitée répond `409` avec toujours le même texte.
    pub async fn a_faire_executer(
        &self,
        tache: Uuid,
        motif: Option<String>,
    ) -> Result<ResultatAction, ApiError> {
        let c = corps(&ExecutionAction {
            confirme: true,
            motif,
        })?;
        self.appel(Methode::Post, &format!("/afaire/{tache}/executer"), Some(c))
            .await
    }
}
