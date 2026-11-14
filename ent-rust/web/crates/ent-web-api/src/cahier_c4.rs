//! Cahier de textes, lot C4 : la charge de travail. Routes `/cahier/c4/*` d'`ent-api` ; types d'`ent-types::cahier_c4`.

use chrono::NaiveDate;
use ent_types::cahier_c4::{
    AlerteChargeDto, AlertesIgnoreesDto, ApercuBody, ApercuChargeDto, CalendrierClasseDto,
    ChargeEleveDto, PositionBody, ReperesDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `POST /cahier/c4/apercu` : la charge des élèves réels au moment de fixer l'échéance.
    pub async fn cdt_charge_apercu(&self, b: &ApercuBody) -> Result<ApercuChargeDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c4/apercu", corps(b)?)
            .await
    }

    /// `POST /cahier/c4/travaux/{id}/alerte-ignoree` : le professeur passe outre ; la dérogation est comptée.
    pub async fn cdt_alerte_ignorer(
        &self,
        travail: Uuid,
    ) -> Result<Vec<AlerteChargeDto>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c4/travaux/{travail}/alerte-ignoree"),
            None,
        )
        .await
    }

    /// `GET /cahier/c4/charge` : la charge d'un élève sur une période.
    pub async fn cdt_charge_eleve(
        &self,
        eleve: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChargeEleveDto, ApiError> {
        let e = eleve.map(|e| format!("eleve_id={e}&")).unwrap_or_default();
        self.appel(
            Methode::Get,
            &format!("/cahier/c4/charge?{e}du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c4/classes/{id}/calendrier` : calendrier d'une classe (équipe, PP, direction).
    pub async fn cdt_calendrier_classe(
        &self,
        classe: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<CalendrierClasseDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c4/classes/{classe}/calendrier?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c4/alertes` : dérogations comptées sur une période.
    pub async fn cdt_alertes_ignorees(
        &self,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<AlertesIgnoreesDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c4/alertes?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c4/reperes` : position et repères de l'établissement.
    pub async fn cdt_reperes(&self) -> Result<ReperesDto, ApiError> {
        self.appel(Methode::Get, "/cahier/c4/reperes", None).await
    }

    /// `PUT /cahier/c4/reperes` : règle la position (direction).
    pub async fn cdt_reperes_regler(&self, position: &str) -> Result<ReperesDto, ApiError> {
        self.appel(
            Methode::Put,
            "/cahier/c4/reperes",
            corps(&PositionBody {
                position: position.into(),
            })?,
        )
        .await
    }
}
