//! Cahier de textes, lot C3 : agenda, « ce que j'ai manqué », coche, résumé hebdomadaire, abonnement au calendrier.
//! Routes `/cahier/c3/*` d'`ent-api` ; types d'`ent-types::cahier_c3`. Jours et heures de l'établissement : aucune
//! conversion ici. La coche n'est jamais servie à un responsable.

use chrono::NaiveDate;
use ent_types::cahier_c3::{
    AbonnementBody, AbonnementCreeDto, AbonnementDto, AgendaDto, CocheBody, ManqueDto,
    ResumeSemaineDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

fn enfant(eleve: Option<Uuid>) -> String {
    eleve.map(|e| format!("eleve_id={e}&")).unwrap_or_default()
}

impl ApiClient {
    /// `GET /cahier/c3/agenda` : travaux, évaluations annoncées et sorties sur une période.
    pub async fn cdt_agenda_eleve(
        &self,
        eleve: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<AgendaDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c3/agenda?{}du={du}&au={au}", enfant(eleve)),
            None,
        )
        .await
    }

    /// `GET /cahier/c3/manque` : le contenu des séances où l'élève était absent.
    pub async fn cdt_manque(
        &self,
        eleve: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ManqueDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c3/manque?{}du={du}&au={au}", enfant(eleve)),
            None,
        )
        .await
    }

    /// `GET /cahier/c3/resume` : la semaine (lundi → dimanche) en décomptes et en lignes.
    pub async fn cdt_resume(
        &self,
        eleve: Option<Uuid>,
        semaine: NaiveDate,
    ) -> Result<ResumeSemaineDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c3/resume?{}semaine={semaine}", enfant(eleve)),
            None,
        )
        .await
    }

    /// `PUT /cahier/c3/travaux/{id}/fait` : l'élève coche (ou décoche) « fait ».
    pub async fn cdt_cocher(&self, travail: Uuid, fait: bool) -> Result<(), ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c3/travaux/{travail}/fait"),
            corps(&CocheBody { fait })?,
        )
        .await
    }

    /// `GET /cahier/c3/abonnements` : abonnements actifs au calendrier (jamais le jeton).
    pub async fn cdt_abonnements(&self) -> Result<Vec<AbonnementDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c3/abonnements", None)
            .await
    }

    /// `POST /cahier/c3/abonnements` : crée un abonnement ; l'adresse n'est donnée qu'une fois.
    pub async fn cdt_abonnement_creer(
        &self,
        eleve: Option<Uuid>,
    ) -> Result<AbonnementCreeDto, ApiError> {
        self.appel(
            Methode::Post,
            "/cahier/c3/abonnements",
            corps(&AbonnementBody { eleve_id: eleve })?,
        )
        .await
    }

    /// `DELETE /cahier/c3/abonnements/{id}` : révoque l'abonnement (le flux cesse aussitôt).
    pub async fn cdt_abonnement_revoquer(&self, id: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/cahier/c3/abonnements/{id}"),
            None,
        )
        .await
    }
}
