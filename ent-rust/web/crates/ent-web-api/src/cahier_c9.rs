//! Cahier de textes, lot C9 : progressions et reconduction. Routes `/cahier/c9/*` d'`ent-api` ; types d'`ent-types::cahier_c9`.

use ent_types::cahier_c9::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c9/progressions`.
    pub async fn cdt_progressions(&self) -> Result<ProgressionsDto, ApiError> {
        self.appel(Methode::Get, "/cahier/c9/progressions", None)
            .await
    }

    /// `POST /cahier/c9/progressions`.
    pub async fn cdt_progression_creer(
        &self,
        b: &ProgressionBody,
    ) -> Result<ProgressionDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c9/progressions", corps(b)?)
            .await
    }

    /// `GET /cahier/c9/progressions/{id}`.
    pub async fn cdt_progression(&self, id: Uuid) -> Result<ProgressionDto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/c9/progressions/{id}"), None)
            .await
    }

    /// `PUT /cahier/c9/progressions/{id}`.
    pub async fn cdt_progression_modifier(
        &self,
        id: Uuid,
        b: &ProgressionBody,
    ) -> Result<ProgressionDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c9/progressions/{id}"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c9/progressions/{id}/partages`.
    pub async fn cdt_progression_partager(
        &self,
        id: Uuid,
        b: &PartageBody,
    ) -> Result<ProgressionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c9/progressions/{id}/partages"),
            corps(b)?,
        )
        .await
    }

    /// `DELETE /cahier/c9/progressions/{id}/partages/{partage}`.
    pub async fn cdt_progression_partage_retirer(
        &self,
        id: Uuid,
        partage: Uuid,
    ) -> Result<ProgressionDto, ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/cahier/c9/progressions/{id}/partages/{partage}"),
            None,
        )
        .await
    }

    /// `POST /cahier/c9/progressions/{id}/planifier`.
    pub async fn cdt_progression_planifier(
        &self,
        id: Uuid,
        b: &PlanifierBody,
    ) -> Result<PlanDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c9/progressions/{id}/planifier"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c9/progressions/{id}/reconduire`.
    pub async fn cdt_progression_reconduire(
        &self,
        id: Uuid,
        b: &ReconduireBody,
    ) -> Result<ProgressionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c9/progressions/{id}/reconduire"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c9/cours/copier`.
    pub async fn cdt_cours_copier(&self, b: &CopierCoursBody) -> Result<CopieCoursDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c9/cours/copier", corps(b)?)
            .await
    }
}
