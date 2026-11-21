//! Service vie scolaire : routes **existantes** d'`ent-api` (absences, justificatifs,
//! signaux d'absence répétée, retards, portail famille). Les droits et les règles
//! métier restent côté serveur ; ce client ne fait que transporter.

use chrono::NaiveDate;
use ent_types::vie_scolaire::{
    AbsenceDto, AbsenceMotifDto, CloreSignalBody, DecisionAbsenceBody, EleveFicheDto,
    EnfantFamilleDto, FamilleResumeDto, JustifierAbsenceBody, RetardDto, SignalAbsenceDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

/// Décision du personnel sur une absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionApi {
    /// `POST /absences/{id}/accepter-justificatif`.
    Accepter,
    /// `POST /absences/{id}/refuser-justificatif`.
    Refuser,
    /// `POST /absences/{id}/annuler-present` (erreur d'appel : l'élève était présent).
    AnnulerPresent,
}

impl DecisionApi {
    pub fn chemin(self, absence_id: Uuid) -> String {
        let fin = match self {
            DecisionApi::Accepter => "accepter-justificatif",
            DecisionApi::Refuser => "refuser-justificatif",
            DecisionApi::AnnulerPresent => "annuler-present",
        };
        format!("/absences/{absence_id}/{fin}")
    }
}

impl ApiClient {
    /// `GET /vs/absences?jour=` : absences **à traiter** couvrant ce jour (personnel
    /// autorisé), avec le nom de l'élève.
    pub async fn absences_a_traiter(&self, jour: NaiveDate) -> Result<Vec<AbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("/vs/absences?jour={jour}"), None)
            .await
    }

    /// `POST /absences/{id}/…` : accepter, refuser ou annuler (retour présent), avec note.
    /// La réponse n'inclut pas le nom de l'élève.
    pub async fn decider_absence(
        &self,
        absence_id: Uuid,
        decision: DecisionApi,
        note: Option<String>,
    ) -> Result<AbsenceDto, ApiError> {
        let c = corps(&DecisionAbsenceBody { note })?;
        self.appel(Methode::Post, &decision.chemin(absence_id), Some(c))
            .await
    }

    /// `GET /vs/signaux` : signaux d'absence répétée non clos.
    pub async fn signaux_absence(&self) -> Result<Vec<SignalAbsenceDto>, ApiError> {
        self.appel(Methode::Get, "/vs/signaux", None).await
    }

    /// `POST /vs/signaux/{id}/prendre`.
    pub async fn prendre_signal(&self, signal_id: Uuid) -> Result<SignalAbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/signaux/{signal_id}/prendre"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /vs/signaux/{id}/clore` (note obligatoire côté serveur).
    pub async fn clore_signal(
        &self,
        signal_id: Uuid,
        note: String,
    ) -> Result<SignalAbsenceDto, ApiError> {
        let c = corps(&CloreSignalBody { note })?;
        self.appel(
            Methode::Post,
            &format!("/vs/signaux/{signal_id}/clore"),
            Some(c),
        )
        .await
    }

    /// `GET /eleves/{id}/absences` (personnel, élève lui-même, parent avec `peut_voir_vs`).
    pub async fn absences_eleve(&self, eleve_id: Uuid) -> Result<Vec<AbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve_id}/absences"), None)
            .await
    }

    /// `GET /eleves/{id}/retards` (200 derniers).
    pub async fn retards_eleve(&self, eleve_id: Uuid) -> Result<Vec<RetardDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve_id}/retards"), None)
            .await
    }

    /// `GET /eleves/{id}` : identité et classe (lecture partielle).
    pub async fn fiche_eleve(&self, eleve_id: Uuid) -> Result<EleveFicheDto, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve_id}"), None)
            .await
    }

    /// `GET /famille/enfants` : enfants du responsable connecté et droits du lien.
    pub async fn enfants_famille(&self) -> Result<Vec<EnfantFamilleDto>, ApiError> {
        self.appel(Methode::Get, "/famille/enfants", None).await
    }

    /// `GET /famille/enfants/{id}/resume` : absences encore sans justification (lecture partielle).
    pub async fn resume_enfant(&self, eleve_id: Uuid) -> Result<FamilleResumeDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/famille/enfants/{eleve_id}/resume"),
            None,
        )
        .await
    }

    /// `GET /motifs-absence` : motifs proposés (personnel et famille).
    pub async fn motifs_absence(&self) -> Result<Vec<AbsenceMotifDto>, ApiError> {
        self.appel(Methode::Get, "/motifs-absence", None).await
    }

    /// `POST /absences/{id}/justifier` : la famille dépose un motif (le personnel tranche ensuite).
    pub async fn justifier_absence(
        &self,
        absence_id: Uuid,
        motif_justif: String,
    ) -> Result<AbsenceDto, ApiError> {
        let c = corps(&JustifierAbsenceBody { motif_justif })?;
        self.appel(
            Methode::Post,
            &format!("/absences/{absence_id}/justifier"),
            Some(c),
        )
        .await
    }

    /// `POST /absences/{id}/annuler-justificatif` : la famille retire sa justification en attente.
    pub async fn retirer_justificatif(&self, absence_id: Uuid) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/absences/{absence_id}/annuler-justificatif"),
            Some(serde_json::json!({})),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins_des_decisions() {
        let id = Uuid::nil();
        assert!(DecisionApi::Accepter
            .chemin(id)
            .ends_with("/accepter-justificatif"));
        assert!(DecisionApi::Refuser
            .chemin(id)
            .ends_with("/refuser-justificatif"));
        assert!(DecisionApi::AnnulerPresent
            .chemin(id)
            .ends_with("/annuler-present"));
    }
}
