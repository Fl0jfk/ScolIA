//! Cahier de textes, lot C7 : adaptations. Routes `/cahier/c7/*` d'`ent-api` ; types d'`ent-types::cahier_c7`.

use chrono::NaiveDate;
use ent_types::cahier_c3::AgendaDto;
use ent_types::cahier_c7::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c7/travaux/{id}/variantes` (professeur en saisie).
    pub async fn cdt_variantes(&self, travail: Uuid) -> Result<Vec<VarianteDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c7/travaux/{travail}/variantes"),
            None,
        )
        .await
    }

    /// `POST /cahier/c7/travaux/{id}/variantes`.
    pub async fn cdt_variante_creer(
        &self,
        travail: Uuid,
        b: &VarianteBody,
    ) -> Result<VarianteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c7/travaux/{travail}/variantes"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c7/variantes/{id}/eleves` : attribue la variante à des élèves nommés.
    pub async fn cdt_variante_attribuer(
        &self,
        variante: Uuid,
        eleves: Vec<Uuid>,
    ) -> Result<VarianteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c7/variantes/{variante}/eleves"),
            corps(&AttributionBody { eleve_ids: eleves })?,
        )
        .await
    }

    /// `DELETE /cahier/c7/variantes/{id}/eleves` : retire la variante à des élèves.
    pub async fn cdt_variante_retirer(
        &self,
        variante: Uuid,
        eleves: Vec<Uuid>,
    ) -> Result<VarianteDto, ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/cahier/c7/variantes/{variante}/eleves"),
            corps(&AttributionBody { eleve_ids: eleves })?,
        )
        .await
    }

    /// `GET /cahier/c7/cours/{id}/pastilles` : faits utiles des élèves du cours.
    pub async fn cdt_pastilles(&self, cours: Uuid) -> Result<PastillesCoursDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c7/cours/{cours}/pastilles"),
            None,
        )
        .await
    }

    /// `POST /cahier/c7/travaux/{id}/amenagements` : aperçu, puis application (`confirmer`).
    pub async fn cdt_amenagements(
        &self,
        travail: Uuid,
        confirmer: bool,
    ) -> Result<ApplicationAmenagementsDto, ApiError> {
        let q = if confirmer { "?confirmer=true" } else { "" };
        self.appel(
            Methode::Post,
            &format!("/cahier/c7/travaux/{travail}/amenagements{q}"),
            None,
        )
        .await
    }

    /// `GET /cahier/c7/aesh/eleves`.
    pub async fn cdt_aesh_eleves(&self) -> Result<Vec<EleveAccompagneDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c7/aesh/eleves", None)
            .await
    }

    /// `GET /cahier/c7/aesh/agenda`.
    pub async fn cdt_aesh_agenda(
        &self,
        eleve: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<AgendaDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c7/aesh/agenda?eleve_id={eleve}&du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `POST /cahier/c7/aesh/notes`.
    pub async fn cdt_aesh_note(
        &self,
        b: &NoteAccompagnementBody,
    ) -> Result<NoteAccompagnementDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c7/aesh/notes", corps(b)?)
            .await
    }

    /// `GET /cahier/c7/affichage` : réglages d'affichage de l'élève.
    pub async fn cdt_affichage(&self) -> Result<AffichageDto, ApiError> {
        self.appel(Methode::Get, "/cahier/c7/affichage", None).await
    }

    /// `PUT /cahier/c7/affichage`.
    pub async fn cdt_affichage_regler(&self, b: &AffichageDto) -> Result<AffichageDto, ApiError> {
        self.appel(Methode::Put, "/cahier/c7/affichage", corps(b)?)
            .await
    }
}
