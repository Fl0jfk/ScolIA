//! Cahier de textes, lot C6 : continuité. Routes `/cahier/c6/*` d'`ent-api` ; types d'`ent-types::cahier_c6`.

use chrono::NaiveDate;
use ent_types::cahier_c6::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c6/reports` : l'écran « vos séances annulées ».
    pub async fn cdt_reports(&self) -> Result<ReportsDto, ApiError> {
        self.appel(Methode::Get, "/cahier/c6/reports", None).await
    }

    /// `POST /cahier/c6/reports/{id}/decision` : accepter (éventuellement vers une autre séance) ou refuser.
    pub async fn cdt_report_decider(
        &self,
        id: Uuid,
        b: &ReportDecisionBody,
    ) -> Result<ReportDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c6/reports/{id}/decision"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c6/reports/lot` : même décision sur plusieurs propositions.
    pub async fn cdt_reports_lot(&self, b: &ReportsLotBody) -> Result<ReportsLotDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c6/reports/lot", corps(b)?)
            .await
    }

    /// `GET /cahier/c6/continuites` : continuités que l'acteur gère.
    pub async fn cdt_continuites(&self) -> Result<Vec<ContinuiteDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c6/continuites", None)
            .await
    }

    /// `POST /cahier/c6/continuites/{id}/cloture` : clore une continuité.
    pub async fn cdt_continuite_clore(&self, id: Uuid, au: NaiveDate) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c6/continuites/{id}/cloture"),
            corps(&ContinuiteCloreBody { au })?,
        )
        .await
    }

    /// `GET /cahier/c6/cours/{id}/bandeau` : élèves en continuité à distance, poste vacant, suppléance.
    pub async fn cdt_cours_bandeau(&self, cours: Uuid) -> Result<BandeauCoursDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c6/cours/{cours}/bandeau"),
            None,
        )
        .await
    }

    /// `POST /cahier/c6/cours/{id}/copies` : décision sur les copies en attente d'une suppléance.
    pub async fn cdt_copies_decider(
        &self,
        cours: Uuid,
        decision: &str,
    ) -> Result<CopiesDecisionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c6/cours/{cours}/copies"),
            corps(&CopiesDecisionBody {
                decision: decision.into(),
            })?,
        )
        .await
    }
}
