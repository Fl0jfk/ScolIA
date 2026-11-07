//! Service du référentiel scolaire : routes `/referentiel/*` d'`ent-api`.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{ApiClient, ApiError, ClasseApi, ClasseDetailApi, Methode, RapportAmorcageApi};

impl ApiClient {
    /// `GET /referentiel/classes` : classes de l'année courante avec effectif au `jour`.
    pub async fn classes(&self, jour: NaiveDate) -> Result<Vec<ClasseApi>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/classes?jour={jour}"),
            None,
        )
        .await
    }

    /// `GET /referentiel/mes-classes` : classes du professeur (ou toutes pour le personnel).
    pub async fn mes_classes(&self, jour: NaiveDate) -> Result<Vec<ClasseApi>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/mes-classes?jour={jour}"),
            None,
        )
        .await
    }

    /// `GET /referentiel/classes/{id}` : élèves affectés, services, groupes.
    pub async fn classe(&self, id: Uuid, jour: NaiveDate) -> Result<ClasseDetailApi, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/classes/{id}?jour={jour}"),
            None,
        )
        .await
    }

    /// `POST /referentiel/amorcer` : rattache l'existant (idempotent).
    pub async fn amorcer_referentiel(&self) -> Result<RapportAmorcageApi, ApiError> {
        self.appel(Methode::Post, "/referentiel/amorcer", None)
            .await
    }
}

impl ApiClient {
    /// `GET /referentiel/classes?annee_id=&jour=` : classes d'une année donnée (bascule d'année).
    pub async fn classes_annee(
        &self,
        annee_id: uuid::Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<ClasseApi>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/classes?annee_id={annee_id}&jour={jour}"),
            None,
        )
        .await
    }
}
