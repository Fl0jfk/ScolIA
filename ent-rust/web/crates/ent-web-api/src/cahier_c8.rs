//! Cahier de textes, lot C8 : tenue, visa, inspection, exports figés, clôture et archive. Routes `/cahier/c8/*` d'`ent-api` ;
//! types d'`ent-types::cahier_c8`.

use chrono::NaiveDate;
use ent_types::cahier_c8::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c8/tenue` (direction).
    pub async fn cdt_tenue(&self, du: NaiveDate, au: NaiveDate) -> Result<TenueDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/tenue?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c8/visas` (direction : tous ; professeur : les siens).
    pub async fn cdt_visas(&self, cours: Option<Uuid>) -> Result<Vec<VisaCahierDto>, ApiError> {
        let chemin = match cours {
            Some(c) => format!("/cahier/c8/visas?cours_id={c}"),
            None => "/cahier/c8/visas".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `POST /cahier/c8/visas`.
    pub async fn cdt_viser(&self, b: &VisaCahierBody) -> Result<VisaCahierDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c8/visas", corps(b)?)
            .await
    }

    /// `GET /cahier/c8/visas/{id}`.
    pub async fn cdt_visa_detail(&self, id: Uuid) -> Result<VisaDetailDto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/c8/visas/{id}"), None)
            .await
    }

    /// `POST /cahier/c8/visas/{id}/observations` (direction).
    pub async fn cdt_observer(&self, visa: Uuid, texte: String) -> Result<VisaCahierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c8/visas/{visa}/observations"),
            corps(&ObservationBody { texte })?,
        )
        .await
    }

    /// `POST /cahier/c8/observations/{id}/reponses` (professeur).
    pub async fn cdt_repondre(
        &self,
        observation: Uuid,
        texte: String,
    ) -> Result<VisaCahierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c8/observations/{observation}/reponses"),
            corps(&ObservationBody { texte })?,
        )
        .await
    }

    /// `GET /cahier/c8/cours/{id}/chronologie` (direction, intervenants).
    pub async fn cdt_chronologie(
        &self,
        cours: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChronologieDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/cours/{cours}/chronologie?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c8/mon-export` (professeur).
    pub async fn cdt_mon_export(&self, annee: Uuid) -> Result<ExportContenuDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/mon-export?annee_id={annee}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c8/inspections`.
    pub async fn cdt_acces_inspection(&self) -> Result<Vec<AccesInspectionDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c8/inspections", None)
            .await
    }

    /// `POST /cahier/c8/inspections`.
    pub async fn cdt_acces_accorder(
        &self,
        b: &AccesInspectionBody,
    ) -> Result<AccesInspectionDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c8/inspections", corps(b)?)
            .await
    }

    /// `POST /cahier/c8/inspections/{id}/prolonger`.
    pub async fn cdt_acces_prolonger(
        &self,
        id: Uuid,
        jours: i32,
    ) -> Result<AccesInspectionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c8/inspections/{id}/prolonger"),
            corps(&ProlongerBody { jours })?,
        )
        .await
    }

    /// `POST /cahier/c8/inspections/{id}/revoquer`.
    pub async fn cdt_acces_revoquer(&self, id: Uuid) -> Result<AccesInspectionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c8/inspections/{id}/revoquer"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `GET /cahier/c8/inspections/{id}/journal` (direction, professeur concerné).
    pub async fn cdt_acces_journal(&self, id: Uuid) -> Result<Vec<ConsultationDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/inspections/{id}/journal"),
            None,
        )
        .await
    }

    /// `GET /cahier/c8/inspection/mes-acces` (inspecteur).
    pub async fn cdt_mes_acces_inspection(&self) -> Result<Vec<AccesInspectionDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c8/inspection/mes-acces", None)
            .await
    }

    /// `GET /cahier/c8/inspection/{acces}/cours/{cours}` (inspecteur : lecture journalisée).
    pub async fn cdt_inspection_lire(
        &self,
        acces: Uuid,
        cours: Uuid,
    ) -> Result<CahierInspectionDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/inspection/{acces}/cours/{cours}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c8/exports` (direction).
    pub async fn cdt_exports(&self) -> Result<Vec<ExportDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c8/exports", None).await
    }

    /// `POST /cahier/c8/exports`.
    pub async fn cdt_export_produire(&self, b: &ExportBody) -> Result<ExportDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c8/exports", corps(b)?)
            .await
    }

    /// `GET /cahier/c8/exports/{id}`.
    pub async fn cdt_export_lire(&self, id: Uuid) -> Result<ExportContenuDto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/c8/exports/{id}"), None)
            .await
    }

    /// `GET /cahier/c8/cloture/controle`.
    pub async fn cdt_cloture_controle(&self, annee: Uuid) -> Result<ControleClotureDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/cloture/controle?annee_id={annee}"),
            None,
        )
        .await
    }

    /// `POST /cahier/c8/cloture`.
    pub async fn cdt_cloturer(&self, b: &ClotureBody) -> Result<ClotureDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c8/cloture", corps(b)?)
            .await
    }

    /// `POST /cahier/c8/cloture/reouvrir`.
    pub async fn cdt_rouvrir(&self, b: &ReouvrirBody) -> Result<ClotureDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c8/cloture/reouvrir", corps(b)?)
            .await
    }

    /// `GET /cahier/c8/archives`.
    pub async fn cdt_archives(&self, annee: Option<Uuid>) -> Result<Vec<ArchiveDto>, ApiError> {
        let chemin = match annee {
            Some(a) => format!("/cahier/c8/archives?annee_id={a}"),
            None => "/cahier/c8/archives".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /cahier/c8/archives/{id}/contenu`.
    pub async fn cdt_archive_contenu(&self, id: Uuid) -> Result<ExportContenuDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c8/archives/{id}/contenu"),
            None,
        )
        .await
    }

    /// `POST /cahier/c8/archives/{id}/decision`.
    pub async fn cdt_archive_decider(
        &self,
        id: Uuid,
        decision: &str,
    ) -> Result<ArchiveDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c8/archives/{id}/decision"),
            corps(&ArchiveDecisionBody {
                decision: decision.into(),
            })?,
        )
        .await
    }
}
