//! Service vie scolaire complète (socle 2, lot S2-4) : journée de la vie scolaire et de l'accueil,
//! déclarations d'absence, relance d'un appel. Routes d'`ent-api/src/vie_scolaire.rs` ; les droits
//! (`absences.declarer`, `appel.verifier`…) sont vérifiés par le serveur.

use chrono::NaiveDate;
use ent_types::vie_scolaire::{
    DeclarationAbsenceDto, DeclarerAbsenceBody, RelancerAppelBody, TableauJourDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /vs/tableau-jour?jour=` : absents, retards, déclarations du jour et vérification de l'appel.
    pub async fn tableau_du_jour(&self, jour: NaiveDate) -> Result<TableauJourDto, ApiError> {
        self.appel(Methode::Get, &format!("/vs/tableau-jour?jour={jour}"), None)
            .await
    }

    /// `POST /vs/appels/relancer` : relance l'enseignant d'une séance dont l'appel n'est pas fait.
    pub async fn relancer_appel(&self, creneau_id: Uuid, jour: NaiveDate) -> Result<(), ApiError> {
        let c = corps(&RelancerAppelBody { creneau_id, jour })?;
        let _: serde_json::Value = self
            .appel(Methode::Post, "/vs/appels/relancer", Some(c))
            .await?;
        Ok(())
    }

    /// `POST /vs/declarations/accueil` : absence annoncée à l'accueil (identité de l'appelant obligatoire).
    pub async fn declarer_absence_accueil(
        &self,
        body: &DeclarerAbsenceBody,
    ) -> Result<DeclarationAbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            "/vs/declarations/accueil",
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /vs/declarations/famille` : absence déclarée par un responsable (ou l'élève majeur).
    pub async fn declarer_absence_famille(
        &self,
        body: &DeclarerAbsenceBody,
    ) -> Result<DeclarationAbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            "/vs/declarations/famille",
            Some(corps(body)?),
        )
        .await
    }

    /// `GET /eleves/{id}/declarations-absence`.
    pub async fn declarations_eleve(
        &self,
        eleve_id: Uuid,
    ) -> Result<Vec<DeclarationAbsenceDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve_id}/declarations-absence"),
            None,
        )
        .await
    }

    /// `POST /vs/declarations/{id}/annuler` (réponse vide acceptée).
    pub async fn annuler_declaration(&self, declaration_id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/vs/declarations/{declaration_id}/annuler"),
                Some(serde_json::json!({})),
            )
            .await?;
        Ok(())
    }
}
