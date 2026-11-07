//! Service d'appel : routes existantes d'`ent-api` (règles métier côté serveur).

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{ApiClient, ApiError, AppelApi, EtatApi, Methode};

impl ApiClient {
    /// `POST /appels` : ouvre l'appel de la séance (idempotent : renvoie l'appel existant).
    pub async fn ouvrir_appel(
        &self,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<AppelApi, ApiError> {
        let corps = serde_json::json!({ "creneau_id": creneau_id, "jour": jour });
        self.appel(Methode::Post, "/appels", Some(corps)).await
    }

    /// `GET /appels/{id}`.
    pub async fn appel_par_id(&self, appel_id: Uuid) -> Result<AppelApi, ApiError> {
        self.appel(Methode::Get, &format!("/appels/{appel_id}"), None)
            .await
    }

    /// `POST /appels/{id}/lignes` : statut d'un élève.
    pub async fn marquer(
        &self,
        appel_id: Uuid,
        eleve_id: Uuid,
        etat: EtatApi,
    ) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "eleve_id": eleve_id, "etat": etat.as_str() });
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/appels/{appel_id}/lignes"),
                Some(corps),
            )
            .await?;
        Ok(())
    }

    /// `POST /appels/{id}/clore` : clôture (transmission à la vie scolaire).
    pub async fn clore(&self, appel_id: Uuid) -> Result<AppelApi, ApiError> {
        self.appel(Methode::Post, &format!("/appels/{appel_id}/clore"), None)
            .await
    }
}
